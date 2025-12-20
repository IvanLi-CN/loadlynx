#![allow(dead_code)]

use core::{fmt::Write as _, str::FromStr, sync::atomic::Ordering};

use alloc::{format, string::String};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{
    Config as NetConfig, DhcpConfig, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4,
    tcp::TcpSocket,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_hal::{peripherals::WIFI, rng::Rng};
use esp_radio::{
    Controller as RadioController, init as radio_init,
    wifi::{self, ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent},
};
use heapless::{String as HString, Vec};
use static_cell::StaticCell;

use loadlynx_protocol::{
    CalKind, FAULT_MCU_OVER_TEMP, FAULT_OVERCURRENT, FAULT_OVERVOLTAGE, FAULT_SINK_OVER_TEMP,
    FastStatus, LimitProfile, PROTOCOL_VERSION, SoftResetReason,
};

use crate::mdns::MdnsConfig;
use crate::{
    ANALOG_FW_VERSION_RAW, CalUartCommand, CalibrationMutex, ENCODER_STEP_MA, ENCODER_VALUE,
    EepromMutex, FAST_STATUS_OK_COUNT, FW_VERSION, HELLO_SEEN, LAST_GOOD_FRAME_MS,
    LIMIT_PROFILE_DEFAULT, LINK_UP, STATE_FLAG_REMOTE_ACTIVE, TARGET_I_MAX_MA, TARGET_I_MIN_MA,
    TelemetryMutex, WIFI_DNS, WIFI_GATEWAY, WIFI_HOSTNAME, WIFI_NETMASK, WIFI_PSK, WIFI_SSID,
    WIFI_STATIC_IP, enqueue_cal_uart, mdns, now_ms32, timestamp_ms, ui::AnalogState,
};

use loadlynx_calibration_format::{self as calfmt, CalPoint, CurveKind, ProfileSource};

/// Shared Wi‑Fi/IPv4 state for future HTTP APIs.
#[derive(Clone, Copy, Debug)]
pub enum WifiConnectionState {
    Idle,
    Connecting,
    Connected,
    Error,
}

#[derive(Clone, Copy, Debug)]
pub enum WifiErrorKind {
    BadStaticConfig,
    ConnectFailed,
    DhcpTimeout,
    LinkLost,
}

#[derive(Clone, Copy, Debug)]
pub struct WifiState {
    pub state: WifiConnectionState,
    pub ipv4: Option<Ipv4Address>,
    pub gateway: Option<Ipv4Address>,
    pub is_static: bool,
    pub last_error: Option<WifiErrorKind>,
    pub mac: Option<[u8; 6]>,
}

impl WifiState {
    const fn new() -> Self {
        Self {
            state: WifiConnectionState::Idle,
            ipv4: None,
            gateway: None,
            is_static: false,
            last_error: None,
            mac: None,
        }
    }
}

#[derive(Clone)]
struct DeviceNames {
    mac: [u8; 6],
    mac_str: HString<17>,
    short_id: HString<6>,
    hostname: HString<32>,
    hostname_fqdn: HString<48>,
}

pub type WifiStateMutex = Mutex<CriticalSectionRawMutex, WifiState>;

static WIFI_STATE_CELL: StaticCell<WifiStateMutex> = StaticCell::new();
static RADIO_CONTROLLER: StaticCell<RadioController<'static>> = StaticCell::new();
// Allow a modest number of simultaneous TCP sockets (HTTP fetches + SSE).
// Value chosen to cover a typical browser's 4–6 parallel GETs without being
// wasteful on RAM.
static NET_RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();

fn derive_device_names(mac: [u8; 6]) -> DeviceNames {
    let short_id = mdns::short_id_from_mac(mac);
    let hostname = mdns::hostname_from_short_id(short_id.as_str());
    let hostname_fqdn = mdns::fqdn_from_hostname(hostname.as_str());
    let mac_str = format_mac(mac);

    DeviceNames {
        mac,
        mac_str,
        short_id,
        hostname,
        hostname_fqdn,
    }
}

fn format_mac(mac: [u8; 6]) -> HString<17> {
    let mut s: HString<17> = HString::new();
    for (idx, byte) in mac.iter().enumerate() {
        let _ = core::write!(s, "{:02x}", byte);
        if idx != mac.len() - 1 {
            let _ = s.push(':');
        }
    }
    s
}

/// Initialize shared Wi‑Fi state storage.
pub fn init_wifi_state() -> &'static WifiStateMutex {
    WIFI_STATE_CELL.init(Mutex::new(WifiState::new()))
}

fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let mut parts = [0u8; 4];
    let mut idx = 0;
    for part in s.split('.') {
        if idx >= 4 {
            return None;
        }
        let v = u8::from_str(part).ok()?;
        parts[idx] = v;
        idx += 1;
    }
    if idx != 4 {
        return None;
    }
    Some(Ipv4Address::new(parts[0], parts[1], parts[2], parts[3]))
}

fn netmask_to_prefix(mask: Ipv4Address) -> Option<u8> {
    let octets = mask.octets();
    let value = u32::from_be_bytes(octets);
    let ones = value.count_ones();
    if ones > 32 {
        return None;
    }
    let prefix = ones as u8;
    let reconstructed = if prefix == 0 {
        0
    } else {
        u32::MAX.checked_shl((32 - prefix as u32) as u32)?
    };
    if reconstructed == value {
        Some(prefix)
    } else {
        None
    }
}

fn build_net_config_from_env() -> (NetConfig, bool) {
    let static_ip = WIFI_STATIC_IP;
    let netmask = WIFI_NETMASK;
    let gateway = WIFI_GATEWAY;

    if let (Some(ip_s), Some(mask_s), Some(gw_s)) = (static_ip, netmask, gateway) {
        if let (Some(ip), Some(mask), Some(gw)) =
            (parse_ipv4(ip_s), parse_ipv4(mask_s), parse_ipv4(gw_s))
        {
            if let Some(prefix) = netmask_to_prefix(mask) {
                let cidr = Ipv4Cidr::new(ip, prefix);
                let mut dns_servers: Vec<Ipv4Address, 3> = Vec::new();

                if let Some(dns_s) = WIFI_DNS {
                    if let Some(dns_ip) = parse_ipv4(dns_s) {
                        let _ = dns_servers.push(dns_ip);
                    }
                }

                let static_cfg = StaticConfigV4 {
                    address: cidr,
                    gateway: Some(gw),
                    dns_servers,
                };

                info!(
                    "Wi-Fi using static IPv4: addr={} prefix={} gw={}",
                    ip, prefix, gw
                );

                return (NetConfig::ipv4_static(static_cfg), true);
            } else {
                warn!(
                    "Wi-Fi static netmask invalid (mask={}); falling back to DHCP",
                    mask
                );
            }
        } else {
            warn!(
                "Wi-Fi static config parse failed (ip={:?}, netmask={:?}, gateway={:?}); falling back to DHCP",
                static_ip, netmask, gateway
            );
        }
    }

    let cfg = NetConfig::dhcpv4(DhcpConfig::default());
    info!("Wi-Fi using DHCPv4 for IPv4 configuration");
    (cfg, false)
}

pub fn spawn_wifi_and_http(
    spawner: &Spawner,
    wifi_peripheral: WIFI<'static>,
    wifi_state: &'static WifiStateMutex,
    telemetry: &'static TelemetryMutex,
    calibration: &'static CalibrationMutex,
    eeprom: &'static EepromMutex,
) {
    // Initialize the shared radio controller once. If the radio init fails, log
    // and gracefully skip Wi‑Fi/HTTP so the rest of the system can run.
    let radio = match radio_init() {
        Ok(ctrl) => ctrl,
        Err(err) => {
            warn!("Wi-Fi radio init failed; disabling Wi-Fi/HTTP: {:?}", err);
            return;
        }
    };
    let radio_ctrl = RADIO_CONTROLLER.init(radio);

    let (wifi_controller, wifi_interfaces) =
        match wifi::new(radio_ctrl, wifi_peripheral, Default::default()) {
            Ok(v) => v,
            Err(err) => {
                warn!(
                    "Wi-Fi driver init (wifi::new) failed; disabling Wi-Fi/HTTP: {:?}",
                    err
                );
                return;
            }
        };

    let wifi_device: WifiDevice<'static> = wifi_interfaces.sta;
    let wifi_mac = wifi_device.mac_address();
    let device_names = derive_device_names(wifi_mac);

    let (net_cfg, is_static) = build_net_config_from_env();

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let resources = NET_RESOURCES.init(StackResources::<6>::new());
    let (stack, runner) = embassy_net::new(wifi_device, net_cfg, resources, seed);

    info!("spawning Wi-Fi connection task");
    spawner
        .spawn(wifi_task(
            wifi_controller,
            stack,
            wifi_state,
            is_static,
            wifi_mac,
        ))
        .expect("wifi_task spawn");

    info!("spawning HTTP workers (count={})", HTTP_WORKER_COUNT);
    for idx in 0..HTTP_WORKER_COUNT {
        spawner
            .spawn(http_worker(
                stack,
                wifi_state,
                telemetry,
                calibration,
                eeprom,
                idx,
            ))
            .expect("http_worker spawn");
    }

    let mdns_cfg = MdnsConfig {
        hostname: device_names.hostname.clone(),
        hostname_fqdn: device_names.hostname_fqdn.clone(),
        port: HTTP_PORT,
    };
    info!(
        "spawning mDNS task (hostname={}, short_id={})",
        device_names.hostname_fqdn.as_str(),
        device_names.short_id.as_str(),
    );
    spawner
        .spawn(mdns::mdns_task(stack, mdns_cfg))
        .expect("mdns_task spawn");

    info!("spawning network stack runner");
    spawner
        .spawn(net_task(runner))
        .expect("net_task runner spawn");
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
async fn wifi_task(
    mut controller: WifiController<'static>,
    stack: Stack<'static>,
    state: &'static WifiStateMutex,
    is_static_ip: bool,
    mac: [u8; 6],
) {
    info!(
        "Wi-Fi task starting (ssid=\"{}\", hostname={:?}, static_ip={})",
        WIFI_SSID, WIFI_HOSTNAME, is_static_ip,
    );

    let ssid = String::from(WIFI_SSID);
    let password = String::from(WIFI_PSK);

    loop {
        {
            let mut guard = state.lock().await;
            guard.state = WifiConnectionState::Connecting;
            guard.last_error = None;
        }

        let client_config = ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(ssid.clone())
                .with_password(password.clone()),
        );

        if !matches!(controller.is_started(), Ok(true)) {
            if let Err(err) = controller.set_config(&client_config) {
                warn!("Wi-Fi set_config error: {:?}", err);
                {
                    let mut guard = state.lock().await;
                    guard.state = WifiConnectionState::Error;
                    guard.last_error = Some(WifiErrorKind::ConnectFailed);
                }
                Timer::after(Duration::from_secs(10)).await;
                continue;
            }

            info!("Starting Wi-Fi STA");
            if let Err(err) = controller.start_async().await {
                warn!("Wi-Fi start_async error: {:?}", err);
                {
                    let mut guard = state.lock().await;
                    guard.state = WifiConnectionState::Error;
                    guard.last_error = Some(WifiErrorKind::ConnectFailed);
                }
                Timer::after(Duration::from_secs(10)).await;
                continue;
            }
        }

        info!("Connecting to Wi-Fi SSID=\"{}\"", WIFI_SSID);
        match controller.connect_async().await {
            Ok(()) => {
                info!("Wi-Fi connect_async returned Ok; waiting for IPv4 config");

                let mut retries: u8 = 0;
                loop {
                    if stack.is_config_up() {
                        break;
                    }
                    if retries >= 30 {
                        warn!("Wi-Fi DHCP/static config not ready within timeout");
                        {
                            let mut guard = state.lock().await;
                            guard.state = WifiConnectionState::Error;
                            guard.last_error = Some(WifiErrorKind::DhcpTimeout);
                        }
                        break;
                    }
                    retries = retries.saturating_add(1);
                    Timer::after(Duration::from_millis(500)).await;
                }

                if !stack.is_config_up() {
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }

                if let Some(cfg) = stack.config_v4() {
                    let ip = cfg.address.address();
                    let gw = cfg.gateway.unwrap_or(Ipv4Address::UNSPECIFIED);
                    info!("Wi-Fi link up: ip={} gw={}", ip, gw);
                    {
                        let mut guard = state.lock().await;
                        guard.state = WifiConnectionState::Connected;
                        guard.ipv4 = Some(ip);
                        guard.gateway = Some(gw);
                        guard.is_static = is_static_ip;
                        guard.last_error = None;
                        guard.mac = Some(mac);
                    }
                }

                // Wait for disconnect; then loop to reconnect.
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                warn!("Wi-Fi STA disconnected; will retry");
                {
                    let mut guard = state.lock().await;
                    guard.state = WifiConnectionState::Error;
                    guard.last_error = Some(WifiErrorKind::LinkLost);
                }
                Timer::after(Duration::from_secs(5)).await;
            }
            Err(err) => {
                warn!("Wi-Fi connect_async error: {:?}", err);
                {
                    let mut guard = state.lock().await;
                    guard.state = WifiConnectionState::Error;
                    guard.last_error = Some(WifiErrorKind::ConnectFailed);
                }
                Timer::after(Duration::from_secs(10)).await;
            }
        }

        // Small cooperative delay between retries.
        Timer::after(Duration::from_millis(100)).await;
    }
}

// Keep at least one worker in accept() even if another is occupied by SSE, avoiding
// browser-side ECONNREFUSED while reusing the same HTTP port and stack.
const HTTP_WORKER_COUNT: usize = 2;
const HTTP_PORT: u16 = 80;

#[embassy_executor::task(pool_size = HTTP_WORKER_COUNT)]
async fn http_worker(
    stack: Stack<'static>,
    state: &'static WifiStateMutex,
    telemetry: &'static TelemetryMutex,
    calibration: &'static CalibrationMutex,
    eeprom: &'static EepromMutex,
    worker_id: usize,
) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    info!("HTTP worker {} starting (port={})", worker_id, HTTP_PORT);

    loop {
        // Ensure network is configured before accepting connections.
        stack.wait_config_up().await;

        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(Duration::from_secs(10)));

        match socket.accept(HTTP_PORT).await {
            Ok(()) => {
                if let Err(err) =
                    handle_http_connection(&mut socket, state, telemetry, calibration, eeprom).await
                {
                    warn!(
                        "HTTP worker {} connection handling error: {:?}",
                        worker_id, err
                    );
                }
            }
            Err(err) => {
                warn!("HTTP worker {} accept error: {:?}", worker_id, err);
                Timer::after(Duration::from_millis(200)).await;
            }
        }

        socket.abort();
    }
}

async fn handle_http_connection(
    socket: &mut TcpSocket<'_>,
    wifi_state: &'static WifiStateMutex,
    telemetry: &'static TelemetryMutex,
    calibration: &'static CalibrationMutex,
    eeprom: &'static EepromMutex,
) -> Result<(), embassy_net::tcp::Error> {
    const MAX_REQUEST_SIZE: usize = 1024;

    let mut buf = [0u8; MAX_REQUEST_SIZE];
    let mut total = 0usize;

    // Read until we see the end of headers or the buffer is full.
    loop {
        let n = socket.read(&mut buf[total..]).await?;
        if n == 0 {
            // Connection closed before any data.
            if total == 0 {
                return Ok(());
            }
            break;
        }
        total += n;
        if total >= MAX_REQUEST_SIZE {
            break;
        }
        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    if total == 0 {
        return Ok(());
    }

    // Parse the request line and headers. To avoid borrowing conflicts with
    // subsequent reads, copy the small method/path/version tokens into owned
    // Strings and only keep indices for the header/body split points.
    let (
        method_s,
        path_s,
        version_s,
        header_end,
        content_length,
        has_content_length,
        accept_event_stream,
    ) = {
        // Try to parse as UTF‑8; fall back to an error on failure.
        let req_str = match core::str::from_utf8(&buf[..total]) {
            Ok(s) => s,
            Err(_) => {
                let mut body = String::new();
                write_error_body(
                    &mut body,
                    "INVALID_REQUEST",
                    "request is not valid UTF-8",
                    false,
                    None,
                );
                write_http_response(socket, "HTTP/1.1", "400 Bad Request", &body).await?;
                return Ok(());
            }
        };

        let mut content_length = 0usize;
        let mut has_content_length = false;
        let mut accept_event_stream = false;

        let mut lines = req_str.lines();
        let request_line = lines.next().unwrap_or("");

        let mut parts = request_line.split_whitespace();
        let method_s = String::from(parts.next().unwrap_or(""));
        let path_s = String::from(parts.next().unwrap_or(""));
        let version_s = String::from(parts.next().unwrap_or("HTTP/1.1"));

        // Locate the end of headers and the beginning of the (optional) body.
        let header_end = match req_str.find("\r\n\r\n") {
            Some(idx) => idx + 4,
            None => {
                let mut body = String::new();
                write_error_body(
                    &mut body,
                    "INVALID_REQUEST",
                    "malformed HTTP headers",
                    false,
                    None,
                );
                write_http_response(socket, "HTTP/1.1", "400 Bad Request", &body).await?;
                return Ok(());
            }
        };

        // Parse headers we care about.
        for line in req_str[..header_end].lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                if let Ok(len) = rest.trim().parse::<usize>() {
                    content_length = len.min(MAX_REQUEST_SIZE);
                    has_content_length = true;
                }
            } else if let Some(rest) = lower.strip_prefix("accept:") {
                if rest.contains("text/event-stream") {
                    accept_event_stream = true;
                }
            }
        }

        (
            method_s,
            path_s,
            version_s,
            header_end,
            content_length,
            has_content_length,
            accept_event_stream,
        )
    };

    let method = method_s.as_str();
    let path = path_s.as_str();
    let version = version_s.as_str();

    // Only HTTP/1.1 is supported.
    if version != "HTTP/1.1" {
        let mut body = String::new();
        write_error_body(
            &mut body,
            "INVALID_REQUEST",
            "only HTTP/1.1 is supported",
            false,
            None,
        );
        write_http_response(socket, "HTTP/1.1", "400 Bad Request", &body).await?;
        return Ok(());
    }

    // Ensure the full body has been read for PUT/POST requests that carry a JSON payload.
    let mut body_str: &str = "";
    if method == "PUT" || method == "POST" {
        if !has_content_length {
            let mut body = String::new();
            write_error_body(
                &mut body,
                "INVALID_REQUEST",
                "missing Content-Length for PUT/POST request",
                false,
                None,
            );
            write_http_response(socket, "HTTP/1.1", "400 Bad Request", &body).await?;
            return Ok(());
        }

        while total < header_end + content_length && total < MAX_REQUEST_SIZE {
            let n = socket.read(&mut buf[total..]).await?;
            if n == 0 {
                break;
            }
            total += n;
        }

        if total < header_end + content_length {
            let mut body = String::new();
            write_error_body(
                &mut body,
                "INVALID_REQUEST",
                "truncated HTTP request body",
                false,
                None,
            );
            write_http_response(socket, "HTTP/1.1", "400 Bad Request", &body).await?;
            return Ok(());
        }

        let raw_body = &buf[header_end..header_end + content_length];
        body_str = core::str::from_utf8(raw_body).unwrap_or("");
    }

    let mut body = String::new();

    // Handle CORS preflight early for any API v1 endpoint. Browsers will send
    // an OPTIONS request when non-simple headers are used (e.g. JSON).
    if method == "OPTIONS" {
        if path.starts_with("/api/v1/") {
            // Empty body is fine; write_http_response already emits the
            // CORS headers we need. Use 200 to satisfy strict preflight
            // checks that expect an \"OK\" status.
            write_http_response(socket, version, "200 OK", "").await?;
            return Ok(());
        }

        write_error_body(
            &mut body,
            "INVALID_REQUEST",
            "unsupported OPTIONS path",
            false,
            None,
        );
        write_http_response(socket, version, "400 Bad Request", &body).await?;
        return Ok(());
    }

    match (method, path) {
        ("GET", "/api/v1/ping") | ("GET", "/health") => {
            body.push_str(r#"{"ok":true}"#);
            write_http_response(socket, version, "200 OK", &body).await?;
        }
        ("GET", "/api/v1/identity") => {
            match render_identity_json(&mut body, wifi_state).await {
                Ok(()) => {
                    write_http_response(socket, version, "200 OK", &body).await?;
                }
                Err(err) => {
                    // render_identity_json already encoded the appropriate ErrorResponse.
                    write_http_response(socket, version, err, &body).await?;
                }
            }
        }
        ("GET", "/api/v1/status") => {
            if accept_event_stream {
                return handle_status_sse(socket, telemetry).await;
            }
            match render_status_json(&mut body, telemetry).await {
                Ok(()) => {
                    write_http_response(socket, version, "200 OK", &body).await?;
                }
                Err(err) => {
                    write_http_response(socket, version, err, &body).await?;
                }
            }
        }
        ("GET", "/api/v1/calibration/profile") => {
            match render_calibration_profile_json(&mut body, calibration).await {
                Ok(()) => write_http_response(socket, version, "200 OK", &body).await?,
                Err(err) => write_http_response(socket, version, err, &body).await?,
            }
        }
        ("POST", "/api/v1/calibration/apply") => {
            match handle_calibration_apply(body_str, &mut body, calibration).await {
                Ok(kind) => {
                    // Immediate UART downlink (multi-chunk CalWrite) is queued onto the UART TX task.
                    if let Err(code) = enqueue_cal_uart(CalUartCommand::SendCurve(kind)) {
                        write_error_body(&mut body, "UNAVAILABLE", code, true, None);
                        write_http_response(socket, version, "503 Service Unavailable", &body)
                            .await?;
                    } else {
                        write_http_response(socket, version, "200 OK", &body).await?;
                    }
                }
                Err(err) => write_http_response(socket, version, err, &body).await?,
            }
        }
        ("POST", "/api/v1/calibration/commit") => {
            match handle_calibration_commit(body_str, &mut body, calibration, eeprom).await {
                Ok(kind) => {
                    if let Err(code) = enqueue_cal_uart(CalUartCommand::SendCurve(kind)) {
                        write_error_body(&mut body, "UNAVAILABLE", code, true, None);
                        write_http_response(socket, version, "503 Service Unavailable", &body)
                            .await?;
                    } else {
                        write_http_response(socket, version, "200 OK", &body).await?;
                    }
                }
                Err(err) => write_http_response(socket, version, err, &body).await?,
            }
        }
        ("POST", "/api/v1/calibration/reset") => {
            match handle_calibration_reset(body_str, &mut body, calibration, eeprom).await {
                Ok(Some(kind)) => {
                    if let Err(code) = enqueue_cal_uart(CalUartCommand::SendCurve(kind)) {
                        write_error_body(&mut body, "UNAVAILABLE", code, true, None);
                        write_http_response(socket, version, "503 Service Unavailable", &body)
                            .await?;
                    } else {
                        write_http_response(socket, version, "200 OK", &body).await?;
                    }
                }
                Ok(None) => {
                    if let Err(code) = enqueue_cal_uart(CalUartCommand::SendAllCurves) {
                        write_error_body(&mut body, "UNAVAILABLE", code, true, None);
                        write_http_response(socket, version, "503 Service Unavailable", &body)
                            .await?;
                    } else {
                        write_http_response(socket, version, "200 OK", &body).await?;
                    }
                }
                Err(err) => write_http_response(socket, version, err, &body).await?,
            }
        }
        ("POST", "/api/v1/calibration/mode") => {
            match handle_calibration_mode(body_str, &mut body, calibration).await {
                Ok(kind) => {
                    if let Err(code) = enqueue_cal_uart(CalUartCommand::SetMode(kind)) {
                        write_error_body(&mut body, "UNAVAILABLE", code, true, None);
                        write_http_response(socket, version, "503 Service Unavailable", &body)
                            .await?;
                    } else {
                        write_http_response(socket, version, "200 OK", &body).await?;
                    }
                }
                Err(err) => write_http_response(socket, version, err, &body).await?,
            }
        }
        ("GET", "/api/v1/cc") => match render_cc_view_json(&mut body, telemetry).await {
            Ok(()) => {
                write_http_response(socket, version, "200 OK", &body).await?;
            }
            Err(err) => {
                write_http_response(socket, version, err, &body).await?;
            }
        },
        ("PUT", "/api/v1/cc") | ("POST", "/api/v1/cc") => {
            match handle_cc_update(body_str, &mut body, telemetry).await {
                Ok(()) => {
                    write_http_response(socket, version, "200 OK", &body).await?;
                }
                Err(err) => {
                    write_http_response(socket, version, err, &body).await?;
                }
            }
        }
        ("POST", "/api/v1/soft-reset") => match handle_soft_reset_http(body_str, &mut body) {
            Ok(()) => {
                write_http_response(socket, version, "200 OK", &body).await?;
            }
            Err(status) => {
                write_http_response(socket, version, status, &body).await?;
            }
        },
        ("GET", _) => {
            write_error_body(&mut body, "UNSUPPORTED_OPERATION", "not found", false, None);
            write_http_response(socket, version, "404 Not Found", &body).await?;
        }
        _ => {
            write_error_body(
                &mut body,
                "INVALID_REQUEST",
                "only GET, PUT, and POST are supported",
                false,
                None,
            );
            write_http_response(socket, version, "400 Bad Request", &body).await?;
        }
    }

    socket.flush().await?;
    Ok(())
}

fn write_json_string_escaped(buf: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if c < ' ' => buf.push('?'),
            c => buf.push(c),
        }
    }
}

fn write_error_body(
    buf: &mut String,
    code: &str,
    message: &str,
    retryable: bool,
    details_json: Option<&str>,
) {
    buf.clear();
    buf.push_str("{\"error\":{\"code\":\"");
    write_json_string_escaped(buf, code);
    buf.push_str("\",\"message\":\"");
    write_json_string_escaped(buf, message);
    buf.push_str("\",\"retryable\":");
    buf.push_str(if retryable { "true" } else { "false" });
    if let Some(details) = details_json {
        buf.push_str(",\"details\":");
        buf.push_str(details);
    }
    buf.push_str("}}");
}

async fn write_http_response(
    socket: &mut TcpSocket<'_>,
    version: &str,
    status_line: &str,
    body: &str,
) -> Result<(), embassy_net::tcp::Error> {
    // Minimal CORS support to allow the LoadLynx web console (running on a
    // separate origin during development) to access the HTTP API.
    const CORS_ALLOW_ORIGIN: &str = "*";
    const CORS_ALLOW_METHODS: &str = "GET, PUT, POST, OPTIONS";
    const CORS_ALLOW_HEADERS: &str = "Content-Type";
    const CORS_ALLOW_PRIVATE_NETWORK: &str = "true";

    let mut head = String::new();
    let _ = core::write!(
        &mut head,
        "{} {}\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Access-Control-Allow-Origin: {}\r\n\
         Access-Control-Allow-Methods: {}\r\n\
         Access-Control-Allow-Headers: {}\r\n\
         Access-Control-Allow-Private-Network: {}\r\n\
         Connection: close\r\n\
         Content-Length: {}\r\n\
         \r\n",
        version,
        status_line,
        CORS_ALLOW_ORIGIN,
        CORS_ALLOW_METHODS,
        CORS_ALLOW_HEADERS,
        CORS_ALLOW_PRIVATE_NETWORK,
        body.as_bytes().len()
    );
    socket.write(head.as_bytes()).await?;
    socket.write(body.as_bytes()).await?;
    Ok(())
}

async fn write_sse_response_head(
    socket: &mut TcpSocket<'_>,
) -> Result<(), embassy_net::tcp::Error> {
    const CORS_ALLOW_ORIGIN: &str = "*";
    const CORS_ALLOW_METHODS: &str = "GET, PUT, POST, OPTIONS";
    const CORS_ALLOW_HEADERS: &str = "Content-Type";
    const CORS_ALLOW_PRIVATE_NETWORK: &str = "true";

    let mut head = String::new();
    let _ = core::write!(
        &mut head,
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Access-Control-Allow-Origin: {}\r\n\
         Access-Control-Allow-Methods: {}\r\n\
         Access-Control-Allow-Headers: {}\r\n\
         Access-Control-Allow-Private-Network: {}\r\n\
         Connection: keep-alive\r\n\
         \r\n",
        CORS_ALLOW_ORIGIN,
        CORS_ALLOW_METHODS,
        CORS_ALLOW_HEADERS,
        CORS_ALLOW_PRIVATE_NETWORK,
    );

    socket.write(head.as_bytes()).await.map(|_| ())
}

/// Render the JSON body for `GET /api/v1/identity`.
///
/// On error this function writes an appropriate ErrorResponse into `buf` and
/// returns the HTTP status line to use.
async fn render_identity_json(
    buf: &mut String,
    wifi_state: &'static WifiStateMutex,
) -> Result<(), &'static str> {
    let wifi = {
        let guard = wifi_state.lock().await;
        *guard
    };

    // If we don't have a usable IPv4 config yet, treat the service as
    // temporarily unavailable.
    if !matches!(wifi.state, WifiConnectionState::Connected) || wifi.ipv4.is_none() {
        write_error_body(buf, "UNAVAILABLE", "Wi-Fi is not connected", true, None);
        return Err("503 Service Unavailable");
    }

    let ip = wifi.ipv4.unwrap();
    let octets = ip.octets();
    let names = wifi.mac.map(derive_device_names);

    buf.clear();
    buf.push('{');

    // device_id: prefer hostname, fall back to a stable placeholder.
    buf.push_str("\"device_id\":\"");
    if let Some(host) = WIFI_HOSTNAME {
        write_json_string_escaped(buf, host);
    } else if let Some(ref names) = names {
        write_json_string_escaped(buf, names.hostname.as_str());
    } else {
        write_json_string_escaped(buf, "llx-digital-01");
    }
    buf.push_str("\",");

    if let Some(ref names) = names {
        buf.push_str("\"hostname\":\"");
        write_json_string_escaped(buf, names.hostname_fqdn.as_str());
        buf.push_str("\",\"short_id\":\"");
        write_json_string_escaped(buf, names.short_id.as_str());
        buf.push_str("\",");
    }

    // digital_fw_version
    buf.push_str("\"digital_fw_version\":\"");
    write_json_string_escaped(buf, FW_VERSION);
    buf.push_str("\",");

    // analog_fw_version: for now expose the compact HELLO fw_version as
    // 0xXXXXXXXX when available; otherwise "unknown".
    buf.push_str("\"analog_fw_version\":\"");
    let analog_raw = ANALOG_FW_VERSION_RAW.load(Ordering::Relaxed);
    if analog_raw != 0 {
        let s = format!("0x{:08x}", analog_raw);
        write_json_string_escaped(buf, &s);
    } else {
        write_json_string_escaped(buf, "unknown");
    }
    buf.push_str("\",");

    // protocol_version
    buf.push_str("\"protocol_version\":");
    let _ = core::write!(buf, "{}", PROTOCOL_VERSION);
    buf.push_str(",");

    // uptime_ms
    buf.push_str("\"uptime_ms\":");
    let _ = core::write!(buf, "{}", timestamp_ms());
    buf.push_str(",");

    // network block
    buf.push_str("\"network\":{");
    // ip
    buf.push_str("\"ip\":\"");
    let _ = core::write!(
        buf,
        "{}.{}.{}.{}",
        octets[0],
        octets[1],
        octets[2],
        octets[3]
    );
    buf.push_str("\",");
    // mac
    buf.push_str("\"mac\":\"");
    if let Some(ref names) = names {
        write_json_string_escaped(buf, names.mac_str.as_str());
    } else {
        write_json_string_escaped(buf, "unknown");
    }
    buf.push_str("\",");
    // hostname
    buf.push_str("\"hostname\":\"");
    if let Some(ref names) = names {
        write_json_string_escaped(buf, names.hostname_fqdn.as_str());
    } else if let Some(host) = WIFI_HOSTNAME {
        write_json_string_escaped(buf, host);
    } else {
        write_json_string_escaped(buf, "loadlynx-digital");
    }
    buf.push_str("\"},");

    // capabilities
    buf.push_str("\"capabilities\":{");
    buf.push_str("\"cc_supported\":true,");
    buf.push_str("\"cv_supported\":false,");
    buf.push_str("\"cp_supported\":false,");
    buf.push_str("\"api_version\":\"1.0.0\"}");

    buf.push('}');
    Ok(())
}

/// Render the JSON body for `GET /api/v1/status` (single-shot snapshot).
async fn render_status_json(
    buf: &mut String,
    telemetry: &'static TelemetryMutex,
) -> Result<(), &'static str> {
    render_status_json_inner(buf, telemetry, false).await
}

async fn render_status_json_sse(
    buf: &mut String,
    telemetry: &'static TelemetryMutex,
) -> Result<(), &'static str> {
    // Allow offline snapshots to keep the SSE stream alive; the consumer can
    // inspect link_up to decide how to render.
    render_status_json_inner(buf, telemetry, true).await
}

async fn render_status_json_inner(
    buf: &mut String,
    telemetry: &'static TelemetryMutex,
    allow_offline: bool,
) -> Result<(), &'static str> {
    // Require the UART link to be up and at least one valid FastStatus frame.
    let link_up = LINK_UP.load(Ordering::Relaxed);
    let fast_ok = FAST_STATUS_OK_COUNT.load(Ordering::Relaxed);
    let last_good = LAST_GOOD_FRAME_MS.load(Ordering::Relaxed);
    let now = now_ms32();
    let age_ms = if last_good == 0 {
        u32::MAX
    } else {
        now.wrapping_sub(last_good)
    };

    if (!link_up || fast_ok == 0) && !allow_offline {
        let details = format!(r#"{{"last_frame_age_ms":{}}}"#, age_ms);
        write_error_body(
            buf,
            "LINK_DOWN",
            "UART link is down or no FastStatus frames received",
            true,
            Some(&details),
        );
        return Err("503 Service Unavailable");
    }

    let (status, analog_state) = {
        let guard = telemetry.lock().await;
        let status = guard.last_status.unwrap_or(FastStatus::default());
        let analog_state = AnalogState::from_u8(crate::ANALOG_STATE.load(Ordering::Relaxed));
        (status, analog_state)
    };

    let hello_seen = HELLO_SEEN.load(Ordering::Relaxed);

    buf.clear();
    buf.push('{');

    // "status": { ... FastStatusJson ... }
    buf.push_str("\"status\":");
    write_fast_status_json(buf, &status);
    buf.push_str(",");

    // link_up / hello_seen
    buf.push_str("\"link_up\":");
    buf.push_str(if link_up { "true" } else { "false" });
    buf.push_str(",\"hello_seen\":");
    buf.push_str(if hello_seen { "true" } else { "false" });

    // analog_state
    buf.push_str(",\"analog_state\":\"");
    let analog_state_str = match analog_state {
        AnalogState::Offline => "offline",
        AnalogState::CalMissing => "cal_missing",
        AnalogState::Faulted => "faulted",
        AnalogState::Ready => "ready",
    };
    write_json_string_escaped(buf, analog_state_str);
    buf.push_str("\",");

    // fault_flags_decoded
    buf.push_str("\"fault_flags_decoded\":[");
    let mut first = true;
    let faults = status.fault_flags;
    if faults & FAULT_OVERCURRENT != 0 {
        if !first {
            buf.push(',');
        }
        buf.push('"');
        write_json_string_escaped(buf, "OVERCURRENT");
        buf.push('"');
        first = false;
    }
    if faults & FAULT_OVERVOLTAGE != 0 {
        if !first {
            buf.push(',');
        }
        buf.push('"');
        write_json_string_escaped(buf, "OVERVOLTAGE");
        buf.push('"');
        first = false;
    }
    if faults & FAULT_MCU_OVER_TEMP != 0 {
        if !first {
            buf.push(',');
        }
        buf.push('"');
        write_json_string_escaped(buf, "MCU_OVER_TEMP");
        buf.push('"');
        first = false;
    }
    if faults & FAULT_SINK_OVER_TEMP != 0 {
        if !first {
            buf.push(',');
        }
        buf.push('"');
        write_json_string_escaped(buf, "SINK_OVER_TEMP");
        buf.push('"');
    }
    buf.push(']');

    buf.push('}');
    Ok(())
}

async fn handle_status_sse(
    socket: &mut TcpSocket<'_>,
    telemetry: &'static TelemetryMutex,
) -> Result<(), embassy_net::tcp::Error> {
    // Conservative rate to keep CPU/stack usage low while still improving
    // smoothness over 400 ms polling.
    const SSE_INTERVAL_MS: u64 = 200;

    write_sse_response_head(socket).await?;

    let mut json_body = String::new();
    let mut frame = String::new();

    loop {
        match render_status_json_sse(&mut json_body, telemetry).await {
            Ok(()) => {
                frame.clear();
                frame.push_str("event: status\r\n");
                frame.push_str("data: ");
                frame.push_str(&json_body);
                frame.push_str("\r\n\r\n");
                socket.write(frame.as_bytes()).await?;
                socket.flush().await?;
            }
            Err(err_status) => {
                frame.clear();
                frame.push_str("event: error\r\n");
                frame.push_str("data: \"");
                write_json_string_escaped(&mut frame, err_status);
                frame.push_str("\"\r\n\r\n");
                socket.write(frame.as_bytes()).await?;
                socket.flush().await?;
                return Ok(());
            }
        }

        Timer::after(Duration::from_millis(SSE_INTERVAL_MS)).await;
    }
}

fn write_fast_status_json(buf: &mut String, status: &FastStatus) {
    buf.push('{');
    let _ = core::write!(buf, "\"uptime_ms\":{}", status.uptime_ms);
    let _ = core::write!(buf, ",\"mode\":{}", status.mode);
    let _ = core::write!(buf, ",\"state_flags\":{}", status.state_flags);
    let _ = core::write!(
        buf,
        ",\"enable\":{}",
        if status.enable { "true" } else { "false" }
    );
    let _ = core::write!(buf, ",\"target_value\":{}", status.target_value);
    let _ = core::write!(buf, ",\"i_local_ma\":{}", status.i_local_ma);
    let _ = core::write!(buf, ",\"i_remote_ma\":{}", status.i_remote_ma);
    let _ = core::write!(buf, ",\"v_local_mv\":{}", status.v_local_mv);
    let _ = core::write!(buf, ",\"v_remote_mv\":{}", status.v_remote_mv);
    let _ = core::write!(buf, ",\"calc_p_mw\":{}", status.calc_p_mw);
    let _ = core::write!(buf, ",\"dac_headroom_mv\":{}", status.dac_headroom_mv);
    let _ = core::write!(buf, ",\"loop_error\":{}", status.loop_error);
    let _ = core::write!(buf, ",\"sink_core_temp_mc\":{}", status.sink_core_temp_mc);
    let _ = core::write!(
        buf,
        ",\"sink_exhaust_temp_mc\":{}",
        status.sink_exhaust_temp_mc
    );
    let _ = core::write!(buf, ",\"mcu_temp_mc\":{}", status.mcu_temp_mc);
    let _ = core::write!(buf, ",\"fault_flags\":{}", status.fault_flags);

    if let Some(v) = status.cal_kind {
        let _ = core::write!(buf, ",\"cal_kind\":{}", v);
    }
    if let Some(v) = status.raw_v_nr_100uv {
        let _ = core::write!(buf, ",\"raw_v_nr_100uv\":{}", v);
    }
    if let Some(v) = status.raw_v_rmt_100uv {
        let _ = core::write!(buf, ",\"raw_v_rmt_100uv\":{}", v);
    }
    if let Some(v) = status.raw_cur_100uv {
        let _ = core::write!(buf, ",\"raw_cur_100uv\":{}", v);
    }
    if let Some(v) = status.raw_dac_code {
        let _ = core::write!(buf, ",\"raw_dac_code\":{}", v);
    }
    buf.push('}');
}

/// Render the JSON body for `GET /api/v1/cc`.
async fn render_cc_view_json(
    buf: &mut String,
    telemetry: &'static TelemetryMutex,
) -> Result<(), &'static str> {
    let link_up = LINK_UP.load(Ordering::Relaxed);
    if !link_up {
        write_error_body(buf, "LINK_DOWN", "UART link is down", true, None);
        return Err("503 Service Unavailable");
    }

    let analog_state = AnalogState::from_u8(crate::ANALOG_STATE.load(Ordering::Relaxed));
    match analog_state {
        AnalogState::Faulted => {
            write_error_body(
                buf,
                "ANALOG_FAULTED",
                "analog board is faulted",
                false,
                None,
            );
            return Err("409 Conflict");
        }
        AnalogState::CalMissing => {
            write_error_body(
                buf,
                "ANALOG_NOT_READY",
                "analog board calibration missing or not ready",
                true,
                None,
            );
            return Err("409 Conflict");
        }
        AnalogState::Offline => {
            write_error_body(buf, "LINK_DOWN", "analog board is offline", true, None);
            return Err("503 Service Unavailable");
        }
        AnalogState::Ready => {}
    }

    let status = {
        let guard = telemetry.lock().await;
        guard.last_status.unwrap_or(FastStatus::default())
    };

    let limit: LimitProfile = LIMIT_PROFILE_DEFAULT;
    let i_total = status.i_local_ma + status.i_remote_ma;
    let v_main = if (status.state_flags & STATE_FLAG_REMOTE_ACTIVE) != 0 {
        status.v_remote_mv
    } else {
        status.v_local_mv
    };

    // Digital-side desired target based on the shared encoder value. This
    // matches the path used by the existing SetPoint TX task and keeps the
    // HTTP view consistent with the local UI knob and remote updates.
    let desired_target = {
        let steps = ENCODER_VALUE.load(Ordering::SeqCst);
        let raw = steps.saturating_mul(ENCODER_STEP_MA);
        raw.clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA)
    };

    buf.clear();
    buf.push('{');
    // enable + target
    buf.push_str("\"enable\":");
    buf.push_str(if status.enable { "true" } else { "false" });
    buf.push_str(",\"target_i_ma\":");
    let _ = core::write!(buf, "{}", desired_target);

    // limit_profile
    buf.push_str(",\"limit_profile\":{");
    let _ = core::write!(buf, "\"max_i_ma\":{}", limit.max_i_ma);
    let _ = core::write!(buf, ",\"max_p_mw\":{}", limit.max_p_mw);
    let _ = core::write!(buf, ",\"ovp_mv\":{}", limit.ovp_mv);
    let _ = core::write!(buf, ",\"temp_trip_mc\":{}", limit.temp_trip_mc);
    let _ = core::write!(buf, ",\"thermal_derate_pct\":{}", limit.thermal_derate_pct);
    buf.push('}');

    // protection config: minimal v0 implementation.
    buf.push_str(",\"protection\":{");
    buf.push_str("\"voltage_mode\":\"protect\",");
    buf.push_str("\"power_mode\":\"protect\"}");

    // Derived measurements
    buf.push_str(",\"i_total_ma\":");
    let _ = core::write!(buf, "{}", i_total);
    buf.push_str(",\"v_main_mv\":");
    let _ = core::write!(buf, "{}", v_main);
    buf.push_str(",\"p_main_mw\":");
    let _ = core::write!(buf, "{}", status.calc_p_mw);

    buf.push('}');
    Ok(())
}

/// Handle `PUT /api/v1/cc`: minimal v0 implementation that accepts `enable`
/// and `target_i_ma` and maps them onto the existing encoder-driven CC
/// control path. Limit fields and protection modes are currently ignored.
async fn handle_cc_update(
    body_in: &str,
    body_out: &mut String,
    telemetry: &'static TelemetryMutex,
) -> Result<(), &'static str> {
    let link_up = LINK_UP.load(Ordering::Relaxed);
    if !link_up {
        write_error_body(body_out, "LINK_DOWN", "UART link is down", true, None);
        return Err("503 Service Unavailable");
    }

    let analog_state = AnalogState::from_u8(crate::ANALOG_STATE.load(Ordering::Relaxed));
    match analog_state {
        AnalogState::Faulted => {
            write_error_body(
                body_out,
                "ANALOG_FAULTED",
                "analog board is faulted",
                false,
                None,
            );
            return Err("409 Conflict");
        }
        AnalogState::CalMissing => {
            write_error_body(
                body_out,
                "ANALOG_NOT_READY",
                "analog board calibration missing or not ready",
                true,
                None,
            );
            return Err("409 Conflict");
        }
        AnalogState::Offline => {
            write_error_body(body_out, "LINK_DOWN", "analog board is offline", true, None);
            return Err("503 Service Unavailable");
        }
        AnalogState::Ready => {}
    }

    let parsed = match parse_cc_update_json(body_in) {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    // Range checks: clamp against the software profile; out-of-range is a hard error.
    let limit = LIMIT_PROFILE_DEFAULT;
    if parsed.target_i_ma < TARGET_I_MIN_MA
        || parsed.target_i_ma > TARGET_I_MAX_MA
        || parsed.target_i_ma > limit.max_i_ma
    {
        let details = format!(
            r#"{{"target_i_ma":{},"max_i_ma":{}}}"#,
            parsed.target_i_ma, limit.max_i_ma
        );
        write_error_body(
            body_out,
            "LIMIT_VIOLATION",
            "target current exceeds allowed range",
            false,
            Some(&details),
        );
        return Err("422 Unprocessable Entity");
    }

    // Map `enable=false` onto a zero-current target; we currently keep the
    // analog-side SetEnable handshake as a separate concern.
    let effective_target = if parsed.enable { parsed.target_i_ma } else { 0 };

    let steps = effective_target / ENCODER_STEP_MA;
    ENCODER_VALUE.store(steps, Ordering::SeqCst);

    // Reuse the GET /cc view to report the updated state back to the caller.
    render_cc_view_json(body_out, telemetry).await
}

struct CcUpdateRequest {
    enable: bool,
    target_i_ma: i32,
}

fn parse_cc_update_json(body: &str) -> Result<CcUpdateRequest, &'static str> {
    let mut enable: Option<bool> = None;
    let mut target_i_ma: Option<i32> = None;

    // Very small hand-written JSON parser: looks for `"enable"` and
    // `"target_i_ma"` keys and extracts their values. This keeps the firmware
    // free from heavy JSON dependencies.
    if let Some(idx) = body.find("\"enable\"") {
        if let Some(colon_idx) = body[idx..].find(':') {
            let value_str = body[idx + colon_idx + 1..].trim_start();
            if value_str.starts_with("true") {
                enable = Some(true);
            } else if value_str.starts_with("false") {
                enable = Some(false);
            } else {
                return Err("enable must be true or false");
            }
        }
    }

    if let Some(idx) = body.find("\"target_i_ma\"") {
        if let Some(colon_idx) = body[idx..].find(':') {
            let mut value_str = body[idx + colon_idx + 1..].trim_start();
            // Strip leading sign/digits until we hit a delimiter.
            let mut end = 0usize;
            for ch in value_str.chars() {
                if ch == '-' || ch.is_ascii_digit() {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            value_str = &value_str[..end];
            match value_str.parse::<i32>() {
                Ok(v) => target_i_ma = Some(v),
                Err(_) => return Err("target_i_ma must be an integer"),
            }
        }
    }

    let enable = enable.ok_or("missing field enable")?;
    let target_i_ma = target_i_ma.ok_or("missing field target_i_ma")?;

    Ok(CcUpdateRequest {
        enable,
        target_i_ma,
    })
}

struct SoftResetRequest<'a> {
    reason_str: &'a str,
    reason: SoftResetReason,
}

fn parse_soft_reset_json(body: &str) -> Result<SoftResetRequest<'_>, &'static str> {
    // Very small hand-written JSON parser: looks for a `"reason"` string field
    // and maps it onto the SoftResetReason enum. This keeps the firmware free
    // from heavy JSON dependencies.
    let idx = body.find("\"reason\"").ok_or("missing field reason")?;
    let colon_idx = body[idx..].find(':').ok_or("malformed reason field")?;
    let mut value_str = body[idx + colon_idx + 1..].trim_start();
    if !value_str.starts_with('"') {
        return Err("reason must be a string");
    }
    value_str = &value_str[1..];
    let end = value_str.find('"').ok_or("reason must be a string")?;
    let reason_str = &value_str[..end];

    let reason = match reason_str {
        "manual" => SoftResetReason::Manual,
        "firmware_update" => SoftResetReason::FirmwareUpdate,
        "ui_recover" => SoftResetReason::UiRecover,
        "link_recover" => SoftResetReason::LinkRecover,
        _ => {
            return Err(
                "reason must be one of \"manual\", \"firmware_update\", \"ui_recover\", \"link_recover\"",
            );
        }
    };

    Ok(SoftResetRequest { reason_str, reason })
}

fn handle_soft_reset_http(body_in: &str, body_out: &mut String) -> Result<(), &'static str> {
    let parsed = match parse_soft_reset_json(body_in) {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    // Require the UART link to be up and at least one valid FastStatus frame.
    let link_up = LINK_UP.load(Ordering::Relaxed);
    let fast_ok = FAST_STATUS_OK_COUNT.load(Ordering::Relaxed);
    let last_good = LAST_GOOD_FRAME_MS.load(Ordering::Relaxed);
    let now = now_ms32();
    let age_ms = if last_good == 0 {
        u32::MAX
    } else {
        now.wrapping_sub(last_good)
    };

    if !link_up || fast_ok == 0 {
        let details = format!(
            r#"{{"last_frame_age_ms":{},"fast_status_ok_count":{}}}"#,
            age_ms, fast_ok
        );
        write_error_body(
            body_out,
            "LINK_DOWN",
            "UART link is down or no FastStatus frames received",
            true,
            Some(&details),
        );
        return Err("503 Service Unavailable");
    }

    if !HELLO_SEEN.load(Ordering::Relaxed) {
        let details = format!(
            r#"{{"last_frame_age_ms":{},"fast_status_ok_count":{}}}"#,
            age_ms, fast_ok
        );
        write_error_body(
            body_out,
            "UNAVAILABLE",
            "analog HELLO not yet observed; soft reset not available",
            true,
            Some(&details),
        );
        return Err("503 Service Unavailable");
    }

    let analog_state = AnalogState::from_u8(crate::ANALOG_STATE.load(Ordering::Relaxed));
    if matches!(analog_state, AnalogState::Offline) {
        write_error_body(body_out, "LINK_DOWN", "analog board is offline", true, None);
        return Err("503 Service Unavailable");
    }

    if let Err(_err) = crate::enqueue_soft_reset(parsed.reason) {
        write_error_body(
            body_out,
            "UNAVAILABLE",
            "soft reset queue is full; try again later",
            true,
            None,
        );
        return Err("503 Service Unavailable");
    }

    body_out.clear();
    body_out.push('{');
    body_out.push_str("\"accepted\":true,\"reason\":\"");
    write_json_string_escaped(body_out, parsed.reason_str);
    body_out.push_str("\"}");

    Ok(())
}

fn ensure_calibration_api_available(body_out: &mut String) -> Result<(), &'static str> {
    let link_up = LINK_UP.load(Ordering::Relaxed);
    if !link_up {
        write_error_body(body_out, "LINK_DOWN", "UART link is down", true, None);
        return Err("503 Service Unavailable");
    }

    let analog_state = AnalogState::from_u8(crate::ANALOG_STATE.load(Ordering::Relaxed));
    match analog_state {
        AnalogState::Faulted => {
            write_error_body(
                body_out,
                "ANALOG_FAULTED",
                "analog board is faulted",
                false,
                None,
            );
            Err("409 Conflict")
        }
        AnalogState::Offline => {
            write_error_body(body_out, "LINK_DOWN", "analog board is offline", true, None);
            Err("503 Service Unavailable")
        }
        AnalogState::CalMissing | AnalogState::Ready => Ok(()),
    }
}

async fn render_calibration_profile_json(
    body_out: &mut String,
    calibration: &'static CalibrationMutex,
) -> Result<(), &'static str> {
    let guard = calibration.lock().await;
    let profile = &guard.profile;

    body_out.clear();
    body_out.push('{');

    body_out.push_str("\"active\":{");
    body_out.push_str("\"source\":\"");
    match profile.source {
        ProfileSource::FactoryDefault => body_out.push_str("factory-default"),
        ProfileSource::UserCalibrated => body_out.push_str("user-calibrated"),
    }
    body_out.push_str("\",\"fmt_version\":");
    let _ = core::write!(body_out, "{}", profile.fmt_version);
    body_out.push_str(",\"hw_rev\":");
    let _ = core::write!(body_out, "{}", profile.hw_rev);
    body_out.push_str("},");

    // current_ch1_points
    body_out.push_str("\"current_ch1_points\":[");
    for (idx, p) in profile.current_ch1.iter().enumerate() {
        if idx != 0 {
            body_out.push(',');
        }
        body_out.push('{');
        let _ = core::write!(
            body_out,
            "\"raw_100uv\":{},\"raw_dac_code\":{},\"meas_ma\":{}",
            p.raw_100uv,
            p.raw_dac_code,
            p.meas_physical
        );
        body_out.push('}');
    }
    body_out.push_str("],");

    // current_ch2_points
    body_out.push_str("\"current_ch2_points\":[");
    for (idx, p) in profile.current_ch2.iter().enumerate() {
        if idx != 0 {
            body_out.push(',');
        }
        body_out.push('{');
        let _ = core::write!(
            body_out,
            "\"raw_100uv\":{},\"raw_dac_code\":{},\"meas_ma\":{}",
            p.raw_100uv,
            p.raw_dac_code,
            p.meas_physical
        );
        body_out.push('}');
    }
    body_out.push_str("],");

    // v_local_points
    body_out.push_str("\"v_local_points\":[");
    for (idx, p) in profile.v_local.iter().enumerate() {
        if idx != 0 {
            body_out.push(',');
        }
        body_out.push('{');
        let _ = core::write!(
            body_out,
            "\"raw_100uv\":{},\"meas_mv\":{}",
            p.raw_100uv,
            p.meas_physical
        );
        body_out.push('}');
    }
    body_out.push_str("],");

    // v_remote_points
    body_out.push_str("\"v_remote_points\":[");
    for (idx, p) in profile.v_remote.iter().enumerate() {
        if idx != 0 {
            body_out.push(',');
        }
        body_out.push('{');
        let _ = core::write!(
            body_out,
            "\"raw_100uv\":{},\"meas_mv\":{}",
            p.raw_100uv,
            p.meas_physical
        );
        body_out.push('}');
    }
    body_out.push_str("]}");

    Ok(())
}

fn parse_string_field<'a>(body: &'a str, key: &str) -> Result<&'a str, &'static str> {
    let needle = match key {
        "kind" => "\"kind\"",
        "points" => "\"points\"",
        _ => return Err("unsupported field"),
    };
    let idx = body.find(needle).ok_or("missing field kind")?;
    let colon = body[idx..].find(':').ok_or("malformed kind field")?;
    let mut s = body[idx + colon + 1..].trim_start();
    if !s.starts_with('"') {
        return Err("kind must be a string");
    }
    s = &s[1..];
    let end = s.find('"').ok_or("kind must be a string")?;
    Ok(&s[..end])
}

fn parse_i32_field(body: &str, key: &str) -> Result<i32, &'static str> {
    let needle = match key {
        "raw_100uv" => "\"raw_100uv\"",
        "raw_dac_code" => "\"raw_dac_code\"",
        "meas_ma" => "\"meas_ma\"",
        "meas_mv" => "\"meas_mv\"",
        _ => return Err("unsupported field"),
    };
    let idx = body.find(needle).ok_or("missing point field")?;
    let colon = body[idx..].find(':').ok_or("malformed point field")?;
    let mut s = body[idx + colon + 1..].trim_start();
    let mut end = 0usize;
    for ch in s.chars() {
        if ch == '-' || ch.is_ascii_digit() {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return Err("point field must be an integer");
    }
    s = &s[..end];
    s.parse::<i32>()
        .map_err(|_| "point field must be an integer")
}

fn parse_curve_kind(kind: &str) -> Result<CurveKind, &'static str> {
    match kind {
        "current_ch1" => Ok(CurveKind::CurrentCh1),
        "current_ch2" => Ok(CurveKind::CurrentCh2),
        "v_local" => Ok(CurveKind::VLocal),
        "v_remote" => Ok(CurveKind::VRemote),
        _ => Err("kind must be one of \"current_ch1\", \"current_ch2\", \"v_local\", \"v_remote\""),
    }
}

fn parse_reset_kind(kind: &str) -> Result<Option<CurveKind>, &'static str> {
    if kind == "all" {
        return Ok(None);
    }
    parse_curve_kind(kind).map(Some)
}

fn parse_cal_mode_kind(kind: &str) -> Result<CalKind, &'static str> {
    match kind {
        "off" => Ok(CalKind::Off),
        "voltage" => Ok(CalKind::Voltage),
        "current_ch1" => Ok(CalKind::CurrentCh1),
        "current_ch2" => Ok(CalKind::CurrentCh2),
        _ => Err("kind must be one of \"off\", \"voltage\", \"current_ch1\", \"current_ch2\""),
    }
}

fn parse_points_array(body: &str) -> Result<&str, &'static str> {
    let idx = body.find("\"points\"").ok_or("missing field points")?;
    let colon = body[idx..].find(':').ok_or("malformed points field")?;
    let mut s = body[idx + colon + 1..].trim_start();
    if !s.starts_with('[') {
        return Err("points must be an array");
    }
    s = &s[1..];
    let end = s.find(']').ok_or("points must be an array")?;
    Ok(&s[..end])
}

fn parse_points_for_kind(kind: CurveKind, body: &str) -> Result<Vec<CalPoint, 24>, &'static str> {
    let arr = parse_points_array(body)?;
    let mut out: Vec<CalPoint, 24> = Vec::new();

    let mut rest = arr;
    while let Some(start) = rest.find('{') {
        rest = &rest[start + 1..];
        let end = rest.find('}').ok_or("malformed points array")?;
        let obj = &rest[..end];
        rest = &rest[end + 1..];

        if obj.trim().is_empty() {
            continue;
        }

        let raw_100uv_i32 = parse_i32_field(obj, "raw_100uv")?;
        if raw_100uv_i32 < i16::MIN as i32 || raw_100uv_i32 > i16::MAX as i32 {
            return Err("raw_100uv out of range for i16");
        }
        let raw_100uv = raw_100uv_i32 as i16;

        let (raw_dac_code, meas_physical) = match kind {
            CurveKind::CurrentCh1 | CurveKind::CurrentCh2 => {
                // Decision: raw_dac_code is required for current points.
                let dac_i32 = parse_i32_field(obj, "raw_dac_code")
                    .map_err(|_| "missing field raw_dac_code for current point")?;
                if dac_i32 < 0 || dac_i32 > u16::MAX as i32 {
                    return Err("raw_dac_code out of range for u16");
                }
                let meas_ma = parse_i32_field(obj, "meas_ma")?;
                (dac_i32 as u16, meas_ma)
            }
            CurveKind::VLocal | CurveKind::VRemote => {
                let meas_mv = parse_i32_field(obj, "meas_mv")?;
                (0u16, meas_mv)
            }
        };

        if out
            .push(CalPoint {
                raw_100uv,
                raw_dac_code,
                meas_physical,
            })
            .is_err()
        {
            return Err("too many points (max 24)");
        }
    }

    if out.is_empty() {
        return Err("points must contain 1..24 items");
    }
    let normalized = calfmt::normalize_points(out);
    if !calfmt::meas_is_strictly_increasing(normalized.as_slice()) {
        return Err("meas must be strictly increasing");
    }
    Ok(normalized)
}

async fn handle_calibration_apply(
    body_in: &str,
    body_out: &mut String,
    calibration: &'static CalibrationMutex,
) -> Result<CurveKind, &'static str> {
    ensure_calibration_api_available(body_out)?;

    let kind_s = match parse_string_field(body_in, "kind") {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };
    let kind = parse_curve_kind(kind_s).map_err(|msg| {
        write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
        "400 Bad Request"
    })?;

    let points = match parse_points_for_kind(kind, body_in) {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    {
        let mut guard = calibration.lock().await;
        *guard.profile.points_for_mut(kind) = points;
        // Apply is RAM-only (no EEPROM write), but the active profile is now user-supplied.
        guard.profile.source = ProfileSource::UserCalibrated;
        guard.profile.fmt_version = calfmt::CAL_FMT_VERSION_LATEST;
    }

    body_out.clear();
    body_out.push_str(r#"{"ok":true}"#);
    Ok(kind)
}

async fn handle_calibration_commit(
    body_in: &str,
    body_out: &mut String,
    calibration: &'static CalibrationMutex,
    eeprom: &'static EepromMutex,
) -> Result<CurveKind, &'static str> {
    ensure_calibration_api_available(body_out)?;

    let kind_s = match parse_string_field(body_in, "kind") {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };
    let kind = match parse_curve_kind(kind_s) {
        Ok(k) => k,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    let points = match parse_points_for_kind(kind, body_in) {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    let (prev, blob) = {
        let mut guard = calibration.lock().await;
        let prev = guard.profile.clone();
        *guard.profile.points_for_mut(kind) = points;
        guard.profile.source = ProfileSource::UserCalibrated;
        guard.profile.fmt_version = calfmt::CAL_FMT_VERSION_LATEST;
        let blob = calfmt::serialize_profile(&guard.profile);
        (prev, blob)
    };

    {
        let mut ep = eeprom.lock().await;
        if let Err(_err) = ep.write_profile_blob(&blob).await {
            let mut guard = calibration.lock().await;
            guard.profile = prev;
            write_error_body(body_out, "UNAVAILABLE", "EEPROM write failed", true, None);
            return Err("503 Service Unavailable");
        }
    }

    body_out.clear();
    body_out.push_str(r#"{"ok":true}"#);
    Ok(kind)
}

fn profile_equals_factory(profile: &calfmt::ActiveProfile) -> bool {
    let factory = calfmt::ActiveProfile::factory_default(calfmt::DIGITAL_HW_REV);
    profile.fmt_version == factory.fmt_version
        && profile.hw_rev == factory.hw_rev
        && profile.current_ch1 == factory.current_ch1
        && profile.current_ch2 == factory.current_ch2
        && profile.v_local == factory.v_local
        && profile.v_remote == factory.v_remote
}

async fn handle_calibration_reset(
    body_in: &str,
    body_out: &mut String,
    calibration: &'static CalibrationMutex,
    eeprom: &'static EepromMutex,
) -> Result<Option<CurveKind>, &'static str> {
    ensure_calibration_api_available(body_out)?;

    let idx = body_in.find("\"kind\"").ok_or_else(|| {
        write_error_body(
            body_out,
            "INVALID_REQUEST",
            "missing field kind",
            false,
            None,
        );
        "400 Bad Request"
    })?;
    let colon = body_in[idx..].find(':').ok_or_else(|| {
        write_error_body(
            body_out,
            "INVALID_REQUEST",
            "malformed kind field",
            false,
            None,
        );
        "400 Bad Request"
    })?;
    let mut s = body_in[idx + colon + 1..].trim_start();
    if !s.starts_with('"') {
        write_error_body(
            body_out,
            "INVALID_REQUEST",
            "kind must be a string",
            false,
            None,
        );
        return Err("400 Bad Request");
    }
    s = &s[1..];
    let end = s.find('"').ok_or_else(|| {
        write_error_body(
            body_out,
            "INVALID_REQUEST",
            "kind must be a string",
            false,
            None,
        );
        "400 Bad Request"
    })?;
    let kind_s = &s[..end];

    let reset_kind = match parse_reset_kind(kind_s) {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    // Apply reset to RAM first, then persist (either clear EEPROM or write updated blob).
    let (factory, should_clear) = {
        let factory = calfmt::ActiveProfile::factory_default(calfmt::DIGITAL_HW_REV);
        let should_clear = reset_kind.is_none();
        (factory, should_clear)
    };

    if should_clear {
        {
            let mut guard = calibration.lock().await;
            guard.profile = factory.clone();
            guard.profile.source = ProfileSource::FactoryDefault;
        }
        {
            let mut ep = eeprom.lock().await;
            if let Err(_err) = ep.clear_profile_blob().await {
                write_error_body(body_out, "UNAVAILABLE", "EEPROM clear failed", true, None);
                return Err("503 Service Unavailable");
            }
        }
        body_out.clear();
        body_out.push_str(r#"{"ok":true}"#);
        return Ok(None);
    }

    let kind = reset_kind.unwrap();
    let blob_or_clear = {
        let mut guard = calibration.lock().await;
        let factory_curve = match kind {
            CurveKind::CurrentCh1 => factory.current_ch1.clone(),
            CurveKind::CurrentCh2 => factory.current_ch2.clone(),
            CurveKind::VLocal => factory.v_local.clone(),
            CurveKind::VRemote => factory.v_remote.clone(),
        };
        *guard.profile.points_for_mut(kind) = factory_curve;

        if profile_equals_factory(&guard.profile) {
            guard.profile = factory.clone();
            guard.profile.source = ProfileSource::FactoryDefault;
            None
        } else {
            guard.profile.source = ProfileSource::UserCalibrated;
            Some(calfmt::serialize_profile(&guard.profile))
        }
    };

    {
        let mut ep = eeprom.lock().await;
        let res = match blob_or_clear {
            None => ep.clear_profile_blob().await,
            Some(ref blob) => ep.write_profile_blob(blob).await,
        };
        if res.is_err() {
            write_error_body(body_out, "UNAVAILABLE", "EEPROM write failed", true, None);
            return Err("503 Service Unavailable");
        }
    }

    body_out.clear();
    body_out.push_str(r#"{"ok":true}"#);
    Ok(Some(kind))
}

async fn handle_calibration_mode(
    body_in: &str,
    body_out: &mut String,
    calibration: &'static CalibrationMutex,
) -> Result<CalKind, &'static str> {
    ensure_calibration_api_available(body_out)?;
    let kind_s = match parse_string_field(body_in, "kind") {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };
    let kind = match parse_cal_mode_kind(kind_s) {
        Ok(v) => v,
        Err(msg) => {
            write_error_body(body_out, "INVALID_REQUEST", msg, false, None);
            return Err("400 Bad Request");
        }
    };

    {
        let mut guard = calibration.lock().await;
        guard.cal_mode = kind;
    }

    body_out.clear();
    body_out.push_str(r#"{"ok":true}"#);
    Ok(kind)
}
