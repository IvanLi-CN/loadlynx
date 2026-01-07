use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_stm32 as stm32;
use embassy_stm32::ucpd::{
    CcPhy, CcPull, CcSel, CcVState, PdPhy, RxError as UcpdRxError, TxError as UcpdTxError,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::Timer;
use heapless::Vec;
use loadlynx_protocol::{CRC_LEN, HEADER_LEN, PROTOCOL_VERSION, crc16_ccitt_false, slip_encode};
use minicbor::encode::write::Cursor;
use minicbor::{Decode, Decoder, Encode, Encoder};
use uom::si::electric_current::milliampere as uom_milliampere;
use uom::si::electric_potential::millivolt as uom_millivolt;
use usbpd::protocol_layer::message::pdo;
use usbpd::protocol_layer::message::request;
use usbpd::protocol_layer::message::units::{ElectricCurrent, ElectricPotential};
use usbpd::sink::device_policy_manager::{DevicePolicyManager, Event};
use usbpd::sink::policy_engine::Sink;
use usbpd::timers::Timer as UsbPdTimer;
use usbpd_traits::{Driver, DriverRxError, DriverTxError};

use embassy_stm32::mode::Async as UartAsync;
use embassy_stm32::usart::UartTx;

pub const MSG_PD_STATUS: u8 = 0x13;
pub const MSG_PD_SINK_REQUEST: u8 = 0x27;

pub const PD_MODE_FIXED: u8 = 0;

pub const PD_TARGET_5V_MV: u32 = 5_000;
pub const PD_TARGET_20V_MV: u32 = 20_000;

pub static PD_DESIRED_TARGET_MV: AtomicU32 = AtomicU32::new(PD_TARGET_5V_MV);
pub static PD_RENEGOTIATE_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

static PD_STATUS_SEQ: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug)]
pub struct FixedPdo {
    pub mv: u32,
    pub max_ma: u32,
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
#[cbor(map)]
pub struct PdSinkRequestPayload {
    #[n(0)]
    pub mode: u8,
    #[n(1)]
    pub target_mv: u32,
}

fn is_cc_attached(v: CcVState) -> bool {
    // For a sink (Rd), a connected source (Rp) should present as HIGH/HIGHEST on the active CC pin.
    matches!(v, CcVState::HIGH | CcVState::HIGHEST)
}

fn select_cc(cc1: CcVState, cc2: CcVState) -> Option<CcSel> {
    match (is_cc_attached(cc1), is_cc_attached(cc2)) {
        (true, false) => Some(CcSel::CC1),
        (false, true) => Some(CcSel::CC2),
        (true, true) => Some(CcSel::CC1),
        (false, false) => None,
    }
}

struct EmbassyTimer;

impl UsbPdTimer for EmbassyTimer {
    fn after_millis(milliseconds: u64) -> impl core::future::Future<Output = ()> {
        Timer::after_millis(milliseconds)
    }
}

struct UcpdDriver<'a> {
    phy: &'a mut PdPhy<'static, stm32::peripherals::UCPD1>,
}

impl<'a> Driver for UcpdDriver<'a> {
    fn wait_for_vbus(&self) -> impl core::future::Future<Output = ()> {
        async {}
    }

    fn receive(
        &mut self,
        buffer: &mut [u8],
    ) -> impl core::future::Future<Output = Result<usize, DriverRxError>> {
        async move {
            match self.phy.receive(buffer).await {
                Ok(size) => Ok(size),
                Err(UcpdRxError::HardReset) => Err(DriverRxError::HardReset),
                Err(UcpdRxError::Crc | UcpdRxError::Overrun) => Err(DriverRxError::Discarded),
            }
        }
    }

    fn transmit(
        &mut self,
        data: &[u8],
    ) -> impl core::future::Future<Output = Result<(), DriverTxError>> {
        async move {
            match self.phy.transmit(data).await {
                Ok(()) => Ok(()),
                Err(UcpdTxError::HardReset) => Err(DriverTxError::HardReset),
                Err(UcpdTxError::Discarded) => Err(DriverTxError::Discarded),
            }
        }
    }

    fn transmit_hard_reset(
        &mut self,
    ) -> impl core::future::Future<Output = Result<(), DriverTxError>> {
        async move {
            match self.phy.transmit_hardreset().await {
                Ok(()) => Ok(()),
                Err(UcpdTxError::HardReset) => Err(DriverTxError::HardReset),
                Err(UcpdTxError::Discarded) => Err(DriverTxError::Discarded),
            }
        }
    }
}

struct AnalogDpm {
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
    fixed_pdos: Vec<FixedPdo, 8>,
    contract_mv: u32,
    contract_ma: u32,
    pending_contract_mv: u32,
    pending_contract_ma: u32,
}

impl AnalogDpm {
    fn new(uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>) -> Self {
        Self {
            uart_tx,
            fixed_pdos: Vec::new(),
            contract_mv: 0,
            contract_ma: 0,
            pending_contract_mv: 0,
            pending_contract_ma: 0,
        }
    }

    fn update_fixed_pdos(&mut self, caps: &pdo::SourceCapabilities) {
        self.fixed_pdos.clear();
        for pdo in caps.pdos().iter() {
            if let pdo::PowerDataObject::FixedSupply(fixed) = pdo {
                let mv = fixed.voltage().get::<uom_millivolt>();
                let max_ma = fixed.max_current().get::<uom_milliampere>();
                let _ = self.fixed_pdos.push(FixedPdo { mv, max_ma });
            }
        }
    }

    fn build_request(&mut self, caps: &pdo::SourceCapabilities) -> request::PowerSource {
        let desired_mv = PD_DESIRED_TARGET_MV.load(Ordering::Relaxed);
        let desired_v = ElectricPotential::new::<uom_millivolt>(desired_mv);

        if desired_mv == PD_TARGET_20V_MV {
            if let Some(selected) =
                request::PowerSource::find_specific_fixed_voltage(caps, desired_v)
            {
                let max_ma = selected.0.max_current().get::<uom_milliampere>();
                let i_req_ma = core::cmp::min(3_000, max_ma);
                let i_req = ElectricCurrent::new::<uom_milliampere>(i_req_ma);

                let req = request::PowerSource::new_fixed_specific(
                    selected,
                    request::CurrentRequest::Specific(i_req),
                )
                .unwrap();

                self.pending_contract_mv = PD_TARGET_20V_MV;
                self.pending_contract_ma = i_req_ma;
                return req;
            }

            warn!(
                "PD desired target {}mV not offered; keeping desired, requesting Safe5V",
                desired_mv
            );
        }

        // Default to Safe5V fixed PDO (index 0 by spec).
        let vsafe = caps.vsafe_5v().unwrap();
        let max_ma = vsafe.max_current().get::<uom_milliampere>();
        let i_req_ma = core::cmp::min(3_000, max_ma);
        let i_req = ElectricCurrent::new::<uom_milliampere>(i_req_ma);
        let req = request::PowerSource::new_fixed(
            request::CurrentRequest::Specific(i_req),
            request::VoltageRequest::Safe5V,
            caps,
        )
        .unwrap();

        self.pending_contract_mv = vsafe.voltage().get::<uom_millivolt>();
        self.pending_contract_ma = i_req_ma;
        req
    }

    async fn send_pd_status(&mut self, attached: bool) {
        let contract_mv = if attached { self.contract_mv } else { 0 };
        let contract_ma = if attached { self.contract_ma } else { 0 };
        let fixed_pdos = if attached {
            Some(&self.fixed_pdos)
        } else {
            None
        };

        let mut raw = [0u8; 192];
        let mut slip = [0u8; 384];

        let seq = PD_STATUS_SEQ.fetch_add(1, Ordering::Relaxed);
        let frame_len = match encode_pd_status_frame(
            seq,
            attached,
            contract_mv,
            contract_ma,
            fixed_pdos,
            &mut raw,
        ) {
            Ok(len) => len,
            Err(e) => {
                let _ = e;
                warn!("PD_STATUS encode failed");
                return;
            }
        };

        let slip_len = match slip_encode(&raw[..frame_len], &mut slip) {
            Ok(len) => len,
            Err(e) => {
                warn!("PD_STATUS SLIP encode failed: {:?}", e);
                return;
            }
        };

        let mut tx = self.uart_tx.lock().await;
        if let Err(e) = tx.write(&slip[..slip_len]).await {
            warn!("PD_STATUS UART write failed: {:?}", e);
        }
    }
}

impl DevicePolicyManager for AnalogDpm {
    fn request(
        &mut self,
        source_capabilities: &pdo::SourceCapabilities,
    ) -> impl core::future::Future<Output = request::PowerSource> {
        async move {
            self.update_fixed_pdos(source_capabilities);
            // Emit PD_STATUS as soon as capabilities are known.
            self.send_pd_status(true).await;
            self.build_request(source_capabilities)
        }
    }

    fn transition_power(
        &mut self,
        _accepted: &request::PowerSource,
    ) -> impl core::future::Future<Output = ()> {
        async move {
            let new_mv = self.pending_contract_mv;
            let new_ma = self.pending_contract_ma;

            let changed = new_mv != self.contract_mv || new_ma != self.contract_ma;
            self.contract_mv = new_mv;
            self.contract_ma = new_ma;

            if changed {
                self.send_pd_status(true).await;
            }
        }
    }

    fn get_event(
        &mut self,
        source_capabilities: &pdo::SourceCapabilities,
    ) -> impl core::future::Future<Output = Event> {
        async move {
            PD_RENEGOTIATE_SIGNAL.wait().await;
            Event::RequestPower(self.build_request(source_capabilities))
        }
    }
}

fn encode_pd_status_frame(
    seq: u8,
    attached: bool,
    contract_mv: u32,
    contract_ma: u32,
    fixed_pdos: Option<&Vec<FixedPdo, 8>>,
    out: &mut [u8],
) -> Result<usize, minicbor::encode::Error<minicbor::encode::write::EndOfSlice>> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(minicbor::encode::Error::message("buffer too small"));
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_PD_STATUS;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut enc = Encoder::new(&mut cursor);

        enc.map(5)?;
        enc.u8(0)?;
        enc.bool(attached)?;
        enc.u8(1)?;
        enc.u32(contract_mv)?;
        enc.u8(2)?;
        enc.u32(contract_ma)?;
        enc.u8(3)?;
        if let Some(list) = fixed_pdos {
            enc.array(list.len() as _)?;
            for pdo in list.iter() {
                enc.array(2)?;
                enc.u32(pdo.mv)?;
                enc.u32(pdo.max_ma)?;
            }
        } else {
            enc.array(0)?;
        }
        enc.u8(4)?;
        enc.array(0)?; // PPS reserved

        cursor.position()
    };

    let len_bytes = (payload_len as u16).to_le_bytes();
    out[4] = len_bytes[0];
    out[5] = len_bytes[1];

    let frame_len_without_crc = HEADER_LEN + payload_len;
    if frame_len_without_crc + CRC_LEN > out.len() {
        return Err(minicbor::encode::Error::message("buffer too small"));
    }
    let crc = crc16_ccitt_false(&out[..frame_len_without_crc]);
    let crc_bytes = crc.to_le_bytes();
    out[frame_len_without_crc] = crc_bytes[0];
    out[frame_len_without_crc + 1] = crc_bytes[1];
    Ok(frame_len_without_crc + CRC_LEN)
}

async fn wait_for_detach(cc_phy: &CcPhy<'static, stm32::peripherals::UCPD1>) {
    loop {
        let (cc1, cc2) = cc_phy.wait_for_vstate_change().await;
        if select_cc(cc1, cc2).is_none() {
            return;
        }
    }
}

#[embassy_executor::task]
pub async fn pd_task(
    mut cc_phy: CcPhy<'static, stm32::peripherals::UCPD1>,
    mut pd_phy: PdPhy<'static, stm32::peripherals::UCPD1>,
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
) -> ! {
    info!("PD task starting (UCPD sink)");

    cc_phy.set_pull(CcPull::Sink);

    // Start in detached state.
    {
        let mut dpm = AnalogDpm::new(uart_tx);
        dpm.send_pd_status(false).await;
    }

    loop {
        // Wait for attach.
        let cc_sel = loop {
            let (cc1, cc2) = cc_phy.vstate();
            if let Some(sel) = select_cc(cc1, cc2) {
                break sel;
            }
            let _ = cc_phy.wait_for_vstate_change().await;
        };

        // Apply active CC selection for PD reception.
        stm32_metapac::UCPD1.cr().modify(|w| w.set_phyccsel(cc_sel));
        info!("PD attached on {:?}", cc_sel);

        // Run a new policy engine instance while attached. Drop it on detach to release the PD PHY borrow.
        loop {
            let driver = UcpdDriver { phy: &mut pd_phy };
            let dpm = AnalogDpm::new(uart_tx);
            let mut sink: Sink<UcpdDriver<'_>, EmbassyTimer, AnalogDpm> = Sink::new(driver, dpm);

            match select(sink.run(), wait_for_detach(&cc_phy)).await {
                Either::First(res) => {
                    warn!("PD sink stopped unexpectedly: {:?}", res);
                    // If still attached, retry after a short delay.
                    Timer::after_millis(100).await;
                }
                Either::Second(()) => {
                    info!("PD detached");
                    let mut dpm = AnalogDpm::new(uart_tx);
                    dpm.send_pd_status(false).await;
                    break;
                }
            }
        }
    }
}

pub fn decode_pd_sink_request_payload(
    payload: &[u8],
) -> Result<PdSinkRequestPayload, minicbor::decode::Error> {
    let mut dec = Decoder::new(payload);
    dec.decode()
}
