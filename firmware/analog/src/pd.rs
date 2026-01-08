use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_stm32 as stm32;
use embassy_stm32::ucpd::{
    CcPhy, CcPull, CcSel, CcVState, Config as UcpdConfig, PdPhy, RxError as UcpdRxError,
    TxError as UcpdTxError, Ucpd,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::{Duration, Timer, with_timeout};
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

fn is_detached(cc1: CcVState, cc2: CcVState) -> bool {
    cc1 == CcVState::LOWEST && cc2 == CcVState::LOWEST
}

fn cc_vstate_to_u8(v: CcVState) -> u8 {
    match v {
        CcVState::LOWEST => 0,
        CcVState::LOW => 1,
        CcVState::HIGH => 2,
        CcVState::HIGHEST => 3,
    }
}

async fn wait_for_attach(cc_phy: &CcPhy<'_, stm32::peripherals::UCPD1>) -> CcSel {
    loop {
        let (cc1, cc2) = cc_phy.vstate();
        if is_detached(cc1, cc2) {
            let _ = cc_phy.wait_for_vstate_change().await;
            continue;
        }

        // Align to the reference example: ensure CC is stable for (at least) tCCDebounce.
        // This avoids picking the wrong CC line due to transient states right after plug-in.
        if with_timeout(Duration::from_millis(100), cc_phy.wait_for_vstate_change())
            .await
            .is_ok()
        {
            continue;
        };

        let (cc1, cc2) = cc_phy.vstate();
        if is_detached(cc1, cc2) {
            continue;
        }

        info!(
            "PD attach detected (stable): cc1={} cc2={}",
            cc_vstate_to_u8(cc1),
            cc_vstate_to_u8(cc2)
        );
        return match (cc1, cc2) {
            (_, CcVState::LOWEST) => CcSel::CC1,
            (CcVState::LOWEST, _) => CcSel::CC2,
            _ => CcSel::CC1, // debug accessory mode / unexpected
        };
    }
}

async fn wait_for_detach(cc_phy: &CcPhy<'_, stm32::peripherals::UCPD1>) {
    loop {
        let (cc1, cc2) = cc_phy.vstate();
        if is_detached(cc1, cc2) {
            return;
        }
        cc_phy.wait_for_vstate_change().await;
    }
}

struct EmbassyTimer;

impl UsbPdTimer for EmbassyTimer {
    fn after_millis(milliseconds: u64) -> impl core::future::Future<Output = ()> {
        Timer::after_millis(milliseconds)
    }
}

struct UcpdDriver<'d> {
    phy: PdPhy<'d, stm32::peripherals::UCPD1>,
    rx_seen: &'d core::sync::atomic::AtomicBool,
    rx_log_budget: u8,
    tx_log_budget: u8,
    req_log_done: bool,
    rx_wait_logged: bool,
}

impl<'d> Driver for UcpdDriver<'d> {
    async fn wait_for_vbus(&self) {}

    async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, DriverRxError> {
        if !self.rx_wait_logged {
            self.rx_wait_logged = true;
            info!("PD RX waiting...");
        }
        match self.phy.receive(buffer).await {
            Ok(size) => {
                self.rx_seen.store(true, Ordering::Relaxed);
                if self.rx_log_budget > 0 {
                    self.rx_log_budget -= 1;
                    if size >= 2 {
                        info!(
                            "PD RX {}B hdr=[0x{:02x},0x{:02x}]",
                            size, buffer[0], buffer[1]
                        );
                    } else {
                        info!("PD RX {}B", size);
                    }
                }
                Ok(size)
            }
            Err(err) => {
                self.rx_seen.store(true, Ordering::Relaxed);
                if self.rx_log_budget > 0 {
                    self.rx_log_budget -= 1;
                    warn!("PD RX err: {:?}", err);
                }
                match err {
                    UcpdRxError::HardReset => Err(DriverRxError::HardReset),
                    UcpdRxError::Crc | UcpdRxError::Overrun => Err(DriverRxError::Discarded),
                }
            }
        }
    }

    async fn transmit(&mut self, data: &[u8]) -> Result<(), DriverTxError> {
        if self.tx_log_budget > 0 {
            self.tx_log_budget -= 1;
            if data.len() >= 2 {
                info!(
                    "PD TX {}B hdr=[0x{:02x},0x{:02x}]",
                    data.len(),
                    data[0],
                    data[1]
                );
            } else {
                info!("PD TX {}B", data.len());
            }
        }
        // Log the first fixed-supply Request (2B header + 4B RDO) per attach session.
        if !self.req_log_done && data.len() == 6 {
            self.req_log_done = true;
            info!("PD TX request bytes={=[u8]:#04x}", data);
        }
        match self.phy.transmit(data).await {
            Ok(()) => Ok(()),
            Err(err) => {
                if self.tx_log_budget > 0 {
                    self.tx_log_budget -= 1;
                    warn!("PD TX err: {:?}", err);
                }
                match err {
                    UcpdTxError::HardReset => Err(DriverTxError::HardReset),
                    UcpdTxError::Discarded => Err(DriverTxError::Discarded),
                }
            }
        }
    }

    async fn transmit_hard_reset(&mut self) -> Result<(), DriverTxError> {
        if self.tx_log_budget > 0 {
            self.tx_log_budget -= 1;
            info!("PD TX hardreset");
        }
        match self.phy.transmit_hardreset().await {
            Ok(()) => Ok(()),
            Err(err) => {
                if self.tx_log_budget > 0 {
                    self.tx_log_budget -= 1;
                    warn!("PD TX hardreset err: {:?}", err);
                }
                match err {
                    UcpdTxError::HardReset => Err(DriverTxError::HardReset),
                    UcpdTxError::Discarded => Err(DriverTxError::Discarded),
                }
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
    followup_desired_request: bool,
    caps_logged: bool,
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
            followup_desired_request: false,
            caps_logged: false,
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

        if !self.caps_logged {
            self.caps_logged = true;
            let mut has_20v = false;
            let mut v5_max_ma = 0u32;
            for p in self.fixed_pdos.iter() {
                if p.mv == PD_TARGET_20V_MV {
                    has_20v = true;
                }
                if p.mv == PD_TARGET_5V_MV {
                    v5_max_ma = p.max_ma;
                }
            }
            info!(
                "PD caps: fixed_pdos={} has_20v={} v5_max_ma={}mA",
                self.fixed_pdos.len(),
                has_20v,
                v5_max_ma
            );
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

    fn build_safe5v_request(&mut self, caps: &pdo::SourceCapabilities) -> request::PowerSource {
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

        info!(
            "PD request: stage=safe5v i_req={}mA (max={}mA)",
            i_req_ma, max_ma
        );
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
    async fn request(
        &mut self,
        source_capabilities: &pdo::SourceCapabilities,
    ) -> request::PowerSource {
        self.update_fixed_pdos(source_capabilities);
        // Emit PD_STATUS as soon as capabilities are known.
        self.send_pd_status(true).await;

        // Some sources are picky when requesting a higher voltage immediately on first contract.
        // Stage the negotiation: establish an explicit Safe5V contract first, then request the
        // desired target (e.g. 20V) from `get_event()` once the policy engine enters Ready.
        let desired_mv = PD_DESIRED_TARGET_MV.load(Ordering::Relaxed);
        if desired_mv == PD_TARGET_20V_MV {
            self.followup_desired_request = true;
            self.build_safe5v_request(source_capabilities)
        } else {
            self.build_request(source_capabilities)
        }
    }

    async fn transition_power(&mut self, _accepted: &request::PowerSource) {
        let new_mv = self.pending_contract_mv;
        let new_ma = self.pending_contract_ma;

        let changed = new_mv != self.contract_mv || new_ma != self.contract_ma;
        self.contract_mv = new_mv;
        self.contract_ma = new_ma;

        if changed {
            self.send_pd_status(true).await;
        }
    }

    async fn get_event(&mut self, source_capabilities: &pdo::SourceCapabilities) -> Event {
        if self.followup_desired_request {
            self.followup_desired_request = false;
            info!("PD request: stage=followup desired");
            return Event::RequestPower(self.build_request(source_capabilities));
        }

        PD_RENEGOTIATE_SIGNAL.wait().await;
        Event::RequestPower(self.build_request(source_capabilities))
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

#[embassy_executor::task]
pub async fn pd_task(
    mut peri: stm32::Peri<'static, stm32::peripherals::UCPD1>,
    mut cc1: stm32::Peri<'static, stm32::peripherals::PB6>,
    mut cc2: stm32::Peri<'static, stm32::peripherals::PB4>,
    mut rx_dma: stm32::Peri<'static, stm32::peripherals::DMA2_CH4>,
    mut tx_dma: stm32::Peri<'static, stm32::peripherals::DMA2_CH5>,
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
) -> ! {
    info!("PD task starting (UCPD sink)");

    // Start in detached state.
    {
        let mut dpm = AnalogDpm::new(uart_tx);
        dpm.send_pd_status(false).await;
    }

    loop {
        // Phase 1: detect attach + "preferred" CC selection.
        let detected_cc_sel = {
            let mut ucpd = Ucpd::new(
                peri.reborrow(),
                super::Irqs,
                cc1.reborrow(),
                cc2.reborrow(),
                UcpdConfig::default(),
            );
            ucpd.cc_phy().set_pull(CcPull::Sink);

            info!("Waiting for USB-PD attach...");
            let sel = wait_for_attach(ucpd.cc_phy()).await;
            match sel {
                CcSel::CC1 => info!("PD attached (detected) on CC1"),
                CcSel::CC2 => info!("PD attached (detected) on CC2"),
            }
            sel
        };

        // Phase 2: run a PD session. If we see PortPartnerUnresponsive with *no RX at all*, try
        // the opposite CC once without requiring physical re-plugging.
        let mut attempt: u8 = 0;
        loop {
            let cc_sel = if attempt == 0 {
                detected_cc_sel
            } else {
                match detected_cc_sel {
                    CcSel::CC1 => CcSel::CC2,
                    CcSel::CC2 => CcSel::CC1,
                }
            };

            let mut ucpd = Ucpd::new(
                peri.reborrow(),
                super::Irqs,
                cc1.reborrow(),
                cc2.reborrow(),
                UcpdConfig::default(),
            );
            ucpd.cc_phy().set_pull(CcPull::Sink);
            // Re-sync attach state after re-initializing UCPD (needed for retry attempt).
            let _ = wait_for_attach(ucpd.cc_phy()).await;

            info!(
                "PD starting session on {:?} (attempt {})",
                cc_sel,
                attempt + 1
            );
            let rx_seen = core::sync::atomic::AtomicBool::new(false);

            // Run the sink while watching for detach. Keep the CC phy alive only within this scope,
            // so we can safely re-initialize UCPD for a retry attempt.
            let run_res = {
                let (cc_phy, pd_phy) =
                    ucpd.split_pd_phy(rx_dma.reborrow(), tx_dma.reborrow(), cc_sel);
                let driver = UcpdDriver {
                    phy: pd_phy,
                    rx_seen: &rx_seen,
                    rx_log_budget: 32,
                    tx_log_budget: 32,
                    req_log_done: false,
                    rx_wait_logged: false,
                };
                let dpm = AnalogDpm::new(uart_tx);
                let mut sink: Sink<UcpdDriver<'_>, EmbassyTimer, AnalogDpm> =
                    Sink::new(driver, dpm);
                select(sink.run(), wait_for_detach(&cc_phy)).await
            };

            match run_res {
                Either::First(res) => {
                    let saw_rx = rx_seen.load(Ordering::Relaxed);
                    warn!("PD sink loop ended: {:?} (rx_seen={})", res, saw_rx);

                    if !saw_rx && attempt == 0 {
                        attempt = 1;
                        Timer::after_millis(50).await;
                        continue;
                    }

                    Timer::after_millis(100).await;
                    break;
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
