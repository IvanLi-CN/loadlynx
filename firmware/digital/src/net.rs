#![allow(dead_code)]

use core::str::FromStr;

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
use heapless::Vec;
use static_cell::StaticCell;

use crate::{
    WIFI_DNS, WIFI_GATEWAY, WIFI_HOSTNAME, WIFI_NETMASK, WIFI_PSK, WIFI_SSID, WIFI_STATIC_IP,
};

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
}

impl WifiState {
    const fn new() -> Self {
        Self {
            state: WifiConnectionState::Idle,
            ipv4: None,
            gateway: None,
            is_static: false,
            last_error: None,
        }
    }
}

pub type WifiStateMutex = Mutex<CriticalSectionRawMutex, WifiState>;

static WIFI_STATE_CELL: StaticCell<WifiStateMutex> = StaticCell::new();
static RADIO_CONTROLLER: StaticCell<RadioController<'static>> = StaticCell::new();
static NET_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

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

    let (net_cfg, is_static) = build_net_config_from_env();

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let resources = NET_RESOURCES.init(StackResources::<3>::new());
    let (stack, runner) = embassy_net::new(wifi_device, net_cfg, resources, seed);

    info!("spawning Wi-Fi connection task");
    spawner
        .spawn(wifi_task(wifi_controller, stack, wifi_state, is_static))
        .expect("wifi_task spawn");

    info!("spawning HTTP server task");
    spawner
        .spawn(http_server_task(stack, wifi_state))
        .expect("http_server_task spawn");

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

const HTTP_PORT: u16 = 80;

#[embassy_executor::task]
async fn http_server_task(stack: Stack<'static>, state: &'static WifiStateMutex) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    info!("HTTP server task starting (port={})", HTTP_PORT);

    loop {
        // Ensure network is configured before accepting connections.
        stack.wait_config_up().await;

        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(Duration::from_secs(10)));

        if let Err(err) = socket.accept(HTTP_PORT).await {
            warn!("HTTP server accept error: {:?}", err);
            Timer::after(Duration::from_secs(1)).await;
            continue;
        }

        if let Err(err) = handle_http_connection(&mut socket, state).await {
            warn!("HTTP connection handling error: {:?}", err);
        }

        socket.abort();
    }
}

async fn handle_http_connection(
    socket: &mut TcpSocket<'_>,
    _state: &'static WifiStateMutex,
) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = [0u8; 512];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let req = core::str::from_utf8(&buf[..n]).unwrap_or("");
    let mut lines = req.lines();
    let request_line = lines.next().unwrap_or("");

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let version = parts.next().unwrap_or("HTTP/1.1");

    let (status_line, body) = if method == "GET" && (path == "/api/v1/ping" || path == "/health") {
        ("200 OK", r#"{"ok":true}"#)
    } else if method == "GET" {
        (
            "404 Not Found",
            r#"{"error":{"code":"UNSUPPORTED_OPERATION","message":"not found","retryable":false}}"#,
        )
    } else {
        (
            "400 Bad Request",
            r#"{"error":{"code":"INVALID_REQUEST","message":"only GET supported","retryable":false}}"#,
        )
    };

    let response = format!(
        "{version} {status}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {len}\r\n\r\n{body}",
        version = version,
        status = status_line,
        len = body.len(),
        body = body
    );

    socket.write(response.as_bytes()).await?;
    socket.flush().await?;
    Ok(())
}
