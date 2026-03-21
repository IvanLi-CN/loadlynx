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
use loadlynx_protocol::{
    EprAvsPdo, EprAvsPdoList, FixedPdo, FixedPdoList, PdStatus, PpsPdo, PpsPdoList,
    encode_pd_status_frame, slip_encode,
};
use uom::si::electric_current::milliampere as uom_milliampere;
use uom::si::electric_potential::millivolt as uom_millivolt;
use uom::si::power::watt as uom_watt;
use usbpd::protocol_layer::message::data::epr_mode;
use usbpd::protocol_layer::message::data::request;
use usbpd::protocol_layer::message::data::sink_capabilities;
use usbpd::protocol_layer::message::data::source_capabilities;
use usbpd::sink::device_policy_manager::{DevicePolicyManager, Event};
use usbpd::sink::policy_engine::Sink;
use usbpd::timers::Timer as UsbPdTimer;
use usbpd::units::{ElectricCurrent, ElectricPotential, Power};
use usbpd_traits::{Driver, DriverRxError, DriverTxError};

use embassy_stm32::mode::Async as UartAsync;
use embassy_stm32::usart::UartTx;

pub const MSG_PD_SINK_REQUEST: u8 = 0x27;

pub const PD_MODE_FIXED: u8 = 0;
pub const PD_MODE_PPS: u8 = 1;
pub const PD_MODE_AVS: u8 = 2;

pub const PD_TARGET_5V_MV: u32 = 5_000;
pub const PD_TARGET_20V_MV: u32 = 20_000;
pub const PD_TARGET_28V_MV: u32 = 28_000;
const PD_EPR_FIXED_28V_OBJECT_POS: u8 = 8;
const PD_EPR_FIXED_28V_MAX_MA: u32 = 5_000;

pub static PD_DESIRED_MODE: AtomicU8 = AtomicU8::new(PD_MODE_FIXED);
pub static PD_DESIRED_OBJECT_POS: AtomicU8 = AtomicU8::new(1);
pub static PD_DESIRED_TARGET_MV: AtomicU32 = AtomicU32::new(PD_TARGET_5V_MV);
pub static PD_DESIRED_I_REQ_MA: AtomicU32 = AtomicU32::new(3_000);
pub static PD_RENEGOTIATE_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

static PD_STATUS_SEQ: AtomicU8 = AtomicU8::new(0);
static PD_STATUS_CACHE: Mutex<CriticalSectionRawMutex, Option<PdStatus>> = Mutex::new(None);

pub async fn cached_pd_status() -> Option<PdStatus> {
    PD_STATUS_CACHE.lock().await.clone()
}

async fn send_pd_status_frame(
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
    status: &PdStatus,
) {
    let mut raw = [0u8; 512];
    let mut slip = [0u8; 1024];

    let seq = PD_STATUS_SEQ.fetch_add(1, Ordering::Relaxed);
    let frame_len = match encode_pd_status_frame(seq, status, &mut raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("PD_STATUS encode failed: {:?}", defmt::Debug2Format(&err));
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

    let mut tx = uart_tx.lock().await;
    if let Err(e) = tx.write(&slip[..slip_len]).await {
        warn!("PD_STATUS UART write failed: {:?}", e);
    }
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
    async fn wait_for_vbus(&mut self) {}

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
    fixed_pdos: FixedPdoList,
    pps_pdos: PpsPdoList,
    epr_avs_pdos: EprAvsPdoList,
    contract_mv: u32,
    contract_ma: u32,
    pending_contract_mv: u32,
    pending_contract_ma: u32,
    epr_active: bool,
    epr_entry_failed: bool,
    followup_desired_request: bool,
    caps_logged: bool,
}

impl AnalogDpm {
    fn new(uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>) -> Self {
        Self {
            uart_tx,
            fixed_pdos: FixedPdoList::new(),
            pps_pdos: PpsPdoList::new(),
            epr_avs_pdos: EprAvsPdoList::new(),
            contract_mv: 0,
            contract_ma: 0,
            pending_contract_mv: 0,
            pending_contract_ma: 0,
            epr_active: false,
            epr_entry_failed: false,
            followup_desired_request: false,
            caps_logged: false,
        }
    }

    fn update_pdos(&mut self, caps: &source_capabilities::SourceCapabilities) {
        self.fixed_pdos.clear();
        self.pps_pdos.clear();
        self.epr_avs_pdos.clear();
        self.epr_active = caps.is_epr_capabilities();

        for (idx, cap) in caps.pdos().iter().enumerate() {
            if cap.is_zero_padding() {
                continue;
            }
            let pos = idx.saturating_add(1) as u8;
            match cap {
                source_capabilities::PowerDataObject::FixedSupply(fixed) => {
                    let mv = fixed.voltage().get::<uom_millivolt>();
                    let max_ma = fixed.max_current().get::<uom_milliampere>();
                    let _ = self.fixed_pdos.push(FixedPdo { pos, mv, max_ma });
                }
                source_capabilities::PowerDataObject::Augmented(aug) => match aug {
                    source_capabilities::Augmented::Spr(spr) => {
                        let min_mv = spr.min_voltage().get::<uom_millivolt>();
                        let max_mv = spr.max_voltage().get::<uom_millivolt>();
                        let max_ma = spr.max_current().get::<uom_milliampere>();
                        let _ = self.pps_pdos.push(PpsPdo {
                            pos,
                            min_mv,
                            max_mv,
                            max_ma,
                        });
                    }
                    source_capabilities::Augmented::Epr(avs) => {
                        let min_mv = avs.min_voltage().get::<uom_millivolt>();
                        let max_mv = avs.max_voltage().get::<uom_millivolt>();
                        let pdp_w = avs.pd_power().get::<uom_watt>() as u16;
                        let _ = self.epr_avs_pdos.push(EprAvsPdo {
                            pos,
                            min_mv,
                            max_mv,
                            pdp_w,
                        });
                    }
                    source_capabilities::Augmented::Unknown(_) => {}
                },
                _ => {}
            }
        }

        self.push_inferred_28v_fixed_pdo(caps);

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
                "PD caps: fixed_pdos={} pps_pdos={} epr_avs_pdos={} epr_active={} has_20v={} v5_max_ma={}mA",
                self.fixed_pdos.len(),
                self.pps_pdos.len(),
                self.epr_avs_pdos.len(),
                self.epr_active,
                has_20v,
                v5_max_ma
            );
        }
    }

    fn push_inferred_28v_fixed_pdo(&mut self, caps: &source_capabilities::SourceCapabilities) {
        if self.epr_active || !caps.epr_mode_capable() {
            return;
        }
        if self
            .fixed_pdos
            .iter()
            .any(|pdo| pdo.pos == PD_EPR_FIXED_28V_OBJECT_POS || pdo.mv == PD_TARGET_28V_MV)
        {
            return;
        }

        // Break the SPR->EPR discovery chicken-and-egg: before EPR entry we only have the SPR
        // Source_Capabilities message, but an EPR-capable source can still surface a read-only
        // inferred 28V rail so the digital UI/API can offer the standard PDO#8 selection.
        let _ = self.fixed_pdos.push(FixedPdo {
            pos: PD_EPR_FIXED_28V_OBJECT_POS,
            mv: PD_TARGET_28V_MV,
            max_ma: PD_EPR_FIXED_28V_MAX_MA,
        });
    }

    fn desired_mode() -> u8 {
        PD_DESIRED_MODE.load(Ordering::Relaxed)
    }

    fn desired_object_pos() -> u8 {
        let object_pos = PD_DESIRED_OBJECT_POS.load(Ordering::Relaxed);
        if object_pos == 0 { 1 } else { object_pos }
    }

    fn desired_target_mv() -> u32 {
        PD_DESIRED_TARGET_MV.load(Ordering::Relaxed)
    }

    fn desired_i_req_ma() -> u32 {
        PD_DESIRED_I_REQ_MA.load(Ordering::Relaxed).max(50)
    }

    fn desired_requires_epr(&self) -> bool {
        match Self::desired_mode() {
            PD_MODE_FIXED => {
                Self::desired_object_pos() >= 8 || Self::desired_target_mv() > PD_TARGET_20V_MV
            }
            PD_MODE_AVS => true,
            _ => false,
        }
    }

    fn desired_epr_operational_pdp(&self, caps: &source_capabilities::SourceCapabilities) -> Power {
        let target_mv = Self::desired_target_mv().max(PD_TARGET_28V_MV);
        let i_req_ma = Self::desired_i_req_ma();
        let desired_watts = target_mv
            .saturating_mul(i_req_ma)
            .div_ceil(1_000_000)
            .max(1);
        let capped_watts = desired_watts.min(source_max_power_w(caps).max(1));
        Power::new::<uom_watt>(capped_watts)
    }

    fn build_request(
        &mut self,
        caps: &source_capabilities::SourceCapabilities,
    ) -> request::PowerSource {
        let desired_mode = Self::desired_mode();
        let object_pos = Self::desired_object_pos();
        let desired_mv = Self::desired_target_mv();
        let desired_i_req_ma = Self::desired_i_req_ma();

        match desired_mode {
            PD_MODE_FIXED => {
                let idx = object_pos.saturating_sub(1) as usize;
                match caps.pdos().get(idx) {
                    Some(source_capabilities::PowerDataObject::FixedSupply(fixed)) => {
                        let mv = fixed.voltage().get::<uom_millivolt>();
                        if desired_mv != 0 && desired_mv != mv {
                            warn!(
                                "PD fixed request mismatch: object_pos={} target_mv={}mV != pdo_mv={}mV (using pdo_mv)",
                                object_pos, desired_mv, mv
                            );
                        }
                        let max_ma = fixed.max_current().get::<uom_milliampere>();
                        let mismatch = desired_i_req_ma > max_ma;
                        let i_req_ma = desired_i_req_ma.min(max_ma);

                        // Fixed request current is expressed in 10mA units (centiampere).
                        let mut raw_current = (i_req_ma / 10) as u16;
                        if raw_current > 0x3ff {
                            warn!("Clamping invalid current: {} mA", 10 * raw_current);
                            raw_current = 0x3ff;
                        }

                        let req = if mv > PD_TARGET_20V_MV {
                            request::PowerSource::EprRequest(request::EprRequestDataObject {
                                rdo: request::FixedVariableSupply(0)
                                    .with_raw_operating_current(raw_current)
                                    .with_raw_max_operating_current(raw_current)
                                    .with_object_position(object_pos)
                                    .with_capability_mismatch(mismatch)
                                    .with_no_usb_suspend(true)
                                    .with_usb_communications_capable(true)
                                    .with_epr_mode_capable(true)
                                    .0,
                                pdo: source_capabilities::PowerDataObject::FixedSupply(*fixed),
                            })
                        } else {
                            request::PowerSource::FixedVariableSupply(
                                request::FixedVariableSupply(0)
                                    .with_raw_operating_current(raw_current)
                                    .with_raw_max_operating_current(raw_current)
                                    .with_object_position(object_pos)
                                    .with_capability_mismatch(mismatch)
                                    .with_no_usb_suspend(true)
                                    .with_usb_communications_capable(true),
                            )
                        };

                        self.pending_contract_mv = mv;
                        self.pending_contract_ma = i_req_ma;
                        return req;
                    }
                    Some(_) => {
                        warn!(
                            "PD fixed request: object_pos={} is not a fixed PDO (fallback Safe5V)",
                            object_pos
                        );
                    }
                    None => {
                        warn!(
                            "PD fixed request: object_pos={} out of range (fallback Safe5V)",
                            object_pos
                        );
                    }
                }
            }
            PD_MODE_PPS => {
                let idx = object_pos.saturating_sub(1) as usize;
                match caps.pdos().get(idx) {
                    Some(source_capabilities::PowerDataObject::Augmented(aug)) => {
                        let source_capabilities::Augmented::Spr(spr) = *aug else {
                            warn!(
                                "PD PPS request: object_pos={} is not a PPS APDO (fallback Safe5V)",
                                object_pos
                            );
                            return self.build_safe5v_request(caps);
                        };
                        let min_mv = spr.min_voltage().get::<uom_millivolt>();
                        let max_mv = spr.max_voltage().get::<uom_millivolt>();
                        let max_ma = spr.max_current().get::<uom_milliampere>();

                        let mut target_mv = desired_mv.clamp(min_mv, max_mv);
                        target_mv = (target_mv / 20).saturating_mul(20);
                        let mut i_req_ma = desired_i_req_ma.min(max_ma);
                        i_req_ma = (i_req_ma / 50).saturating_mul(50).max(50);

                        // PPS request fields are expressed in 20mV/50mA units.
                        let raw_voltage = (target_mv / 20) as u16;
                        let raw_current = (i_req_ma / 50) as u16;

                        let req = request::PowerSource::Pps(
                            request::Pps(0)
                                .with_raw_output_voltage(raw_voltage)
                                .with_raw_operating_current(raw_current)
                                .with_object_position(object_pos)
                                .with_capability_mismatch(false)
                                .with_no_usb_suspend(true)
                                .with_usb_communications_capable(true),
                        );

                        self.pending_contract_mv = target_mv;
                        self.pending_contract_ma = i_req_ma;
                        return req;
                    }
                    Some(_) => {
                        warn!(
                            "PD PPS request: object_pos={} is not a PPS APDO (fallback Safe5V)",
                            object_pos
                        );
                    }
                    None => {
                        warn!(
                            "PD PPS request: object_pos={} out of range (fallback Safe5V)",
                            object_pos
                        );
                    }
                }
            }
            PD_MODE_AVS => {
                let voltage = ElectricPotential::new::<uom_millivolt>(desired_mv);
                let current = ElectricCurrent::new::<uom_milliampere>(desired_i_req_ma);
                match request::PowerSource::new_epr_avs(
                    request::CurrentRequest::Specific(current),
                    voltage,
                    caps,
                ) {
                    Ok(req) => {
                        self.pending_contract_mv = desired_mv;
                        self.pending_contract_ma = desired_i_req_ma;
                        return req;
                    }
                    Err(_) => {
                        warn!(
                            "PD AVS request: target_mv={}mV not covered by EPR AVS caps (fallback Safe5V)",
                            desired_mv
                        );
                    }
                }
            }
            _ => {
                warn!(
                    "PD sink request: unsupported mode={} (fallback Safe5V)",
                    desired_mode
                );
            }
        }

        self.build_safe5v_request(caps)
    }

    fn build_safe5v_request(
        &mut self,
        caps: &source_capabilities::SourceCapabilities,
    ) -> request::PowerSource {
        let vsafe = caps.vsafe_5v().unwrap();
        let max_ma = vsafe.max_current().get::<uom_milliampere>();
        let desired_i_req_ma = PD_DESIRED_I_REQ_MA.load(Ordering::Relaxed).max(50);
        let i_req_ma = desired_i_req_ma.min(max_ma);
        let i_req = ElectricCurrent::new::<uom_milliampere>(i_req_ma);
        let req = request::PowerSource::new_fixed(
            request::CurrentRequest::Specific(i_req),
            request::VoltageRequest::Safe5V,
            caps,
        )
        .unwrap();
        let req = if self.desired_requires_epr() {
            match req {
                request::PowerSource::FixedVariableSupply(rdo) => {
                    request::PowerSource::FixedVariableSupply(rdo.with_epr_mode_capable(true))
                }
                other => other,
            }
        } else {
            req
        };

        info!(
            "PD request: stage=safe5v i_req={}mA (max={}mA) epr_capable={}",
            i_req_ma,
            max_ma,
            self.desired_requires_epr()
        );
        self.pending_contract_mv = vsafe.voltage().get::<uom_millivolt>();
        self.pending_contract_ma = i_req_ma;
        req
    }

    async fn send_pd_status(&mut self, attached: bool) {
        let status = if attached {
            PdStatus {
                attached,
                contract_mv: self.contract_mv,
                contract_ma: self.contract_ma,
                fixed_pdos: self.fixed_pdos.clone(),
                pps_pdos: self.pps_pdos.clone(),
                epr_active: self.epr_active,
                epr_avs_pdos: self.epr_avs_pdos.clone(),
            }
        } else {
            PdStatus {
                attached,
                ..PdStatus::default()
            }
        };

        {
            let mut guard = PD_STATUS_CACHE.lock().await;
            *guard = Some(status.clone());
        }
        send_pd_status_frame(self.uart_tx, &status).await;
    }
}

impl DevicePolicyManager for AnalogDpm {
    async fn request(
        &mut self,
        source_capabilities: &source_capabilities::SourceCapabilities,
    ) -> request::PowerSource {
        self.update_pdos(source_capabilities);
        // Emit PD_STATUS as soon as capabilities are known.
        self.send_pd_status(true).await;

        // Stage the negotiation on initial attach: establish an explicit Safe5V contract first,
        // then request the desired policy from `get_event()` once the policy engine enters Ready.
        //
        // Some sources are picky when requesting higher voltages (or PPS) immediately.
        let desired_mode = Self::desired_mode();
        let desired_mv = Self::desired_target_mv();
        let desired_pos = Self::desired_object_pos();
        let stage_safe5v =
            desired_mode != PD_MODE_FIXED || desired_mv != PD_TARGET_5V_MV || desired_pos != 1;

        if stage_safe5v {
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

    async fn get_event(
        &mut self,
        source_capabilities: &source_capabilities::SourceCapabilities,
    ) -> Event {
        if self.followup_desired_request {
            self.followup_desired_request = false;
            if self.desired_requires_epr() && !source_capabilities.is_epr_capabilities() {
                if self.epr_entry_failed {
                    info!("PD request: retrying EPR entry after a fresh followup request");
                    self.epr_entry_failed = false;
                }
                if source_capabilities.epr_mode_capable() {
                    let pdp = self.desired_epr_operational_pdp(source_capabilities);
                    info!(
                        "PD request: stage=followup enter-epr pdp={}W",
                        pdp.get::<uom_watt>()
                    );
                    return Event::EnterEprMode(pdp);
                }
                warn!("PD request: EPR target requested but source is not EPR capable");
                return Event::None;
            }

            info!("PD request: stage=followup desired");
            return Event::RequestPower(self.build_request(source_capabilities));
        }

        PD_RENEGOTIATE_SIGNAL.wait().await;

        if self.epr_active && !self.desired_requires_epr() {
            info!("PD request: exit EPR for SPR target");
            return Event::ExitEprMode;
        }

        if !self.epr_active && self.desired_requires_epr() {
            if self.epr_entry_failed {
                info!("PD request: retrying EPR entry after explicit renegotiation");
                self.epr_entry_failed = false;
            }
            if source_capabilities.epr_mode_capable() {
                let pdp = self.desired_epr_operational_pdp(source_capabilities);
                info!("PD request: enter EPR pdp={}W", pdp.get::<uom_watt>());
                return Event::EnterEprMode(pdp);
            }
            warn!("PD request: EPR target requested but source is not EPR capable");
            return Event::None;
        }

        Event::RequestPower(self.build_request(source_capabilities))
    }

    async fn epr_mode_entry_failed(&mut self, reason: epr_mode::DataEnterFailed) {
        self.epr_active = false;
        self.epr_entry_failed = true;
        warn!("PD EPR mode entry failed: {:?}", reason);
        self.send_pd_status(true).await;
    }

    fn sink_capabilities(&self) -> sink_capabilities::SinkCapabilities {
        let mut caps = sink_capabilities::SinkCapabilities::new_vsafe5v_only(
            (Self::desired_i_req_ma() / 10).min(0x3ff) as u16,
        );
        if self.desired_requires_epr()
            && let Some(sink_capabilities::SinkPowerDataObject::FixedSupply(fixed)) =
                caps.0.first_mut()
        {
            *fixed = fixed.with_higher_capability(true);
        }
        caps
    }
}

fn source_max_power_w(caps: &source_capabilities::SourceCapabilities) -> u32 {
    caps.pdos()
        .iter()
        .filter_map(|pdo| match pdo {
            source_capabilities::PowerDataObject::FixedSupply(fixed) => Some(
                fixed
                    .voltage()
                    .get::<uom_millivolt>()
                    .saturating_mul(fixed.max_current().get::<uom_milliampere>())
                    / 1_000_000,
            ),
            source_capabilities::PowerDataObject::Augmented(
                source_capabilities::Augmented::Spr(spr),
            ) => Some(
                spr.max_voltage()
                    .get::<uom_millivolt>()
                    .saturating_mul(spr.max_current().get::<uom_milliampere>())
                    / 1_000_000,
            ),
            _ => None,
        })
        .max()
        .unwrap_or(0)
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
            let mut ucpd = Ucpd::new(
                peri.reborrow(),
                super::Irqs,
                cc1.reborrow(),
                cc2.reborrow(),
                UcpdConfig::default(),
            );
            ucpd.cc_phy().set_pull(CcPull::Sink);
            // Re-sync attach state after re-initializing UCPD (needed for retry attempts).
            // NOTE: Use the observed attach orientation to configure the PD PHY. Some sources
            // appear sensitive to orientation mismatches and will not respond if we choose the
            // wrong CC line.
            let attached_cc_sel = wait_for_attach(ucpd.cc_phy()).await;
            let cc_sel = if attempt == 0 {
                attached_cc_sel
            } else {
                match attached_cc_sel {
                    CcSel::CC1 => CcSel::CC2,
                    CcSel::CC2 => CcSel::CC1,
                }
            };
            info!(
                "PD starting session on {:?} (attempt {}, detected={:?}, attached={:?})",
                cc_sel,
                attempt + 1,
                detected_cc_sel,
                attached_cc_sel
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
                        // Brief backoff before retrying on the other CC.
                        Timer::after_millis(100).await;
                        continue;
                    }

                    // Back off before re-entering the attach loop to avoid hammering the source.
                    Timer::after_millis(200).await;
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
