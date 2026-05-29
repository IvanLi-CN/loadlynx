use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use loadlynx_devd::{
    DEFAULT_DEVD_URL, TargetKind, list_digital_usb_port_candidates, write_default_digital_usb_port,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Parser)]
#[command(name = "loadlynx")]
#[command(about = "LoadLynx LAN/USB/devd control CLI")]
struct Cli {
    #[arg(long, default_value = DEFAULT_DEVD_URL)]
    devd: String,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Discover {
        #[arg(long)]
        mdns: bool,
        #[arg(long)]
        lan_scan: bool,
    },
    Devices,
    Status {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
    },
    Flash {
        target: BoardTarget,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long = "no-dry-run", default_value_t = true, action = ArgAction::SetFalse)]
        dry_run: bool,
    },
    Reset {
        target: BoardTarget,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long = "no-dry-run", default_value_t = true, action = ArgAction::SetFalse)]
        dry_run: bool,
    },
    Monitor {
        target: BoardTarget,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long, default_value_t = 200)]
        tail: usize,
        #[arg(long, value_enum, default_value_t = MonitorFormat::Human)]
        format: MonitorFormat,
    },
    Output {
        #[command(subcommand)]
        command: OutputCommand,
    },
    UsbPort {
        #[command(subcommand)]
        command: UsbPortCommand,
    },
    Hardware {
        #[command(subcommand)]
        command: HardwareCommand,
    },
}

#[derive(Debug, Subcommand)]
enum OutputCommand {
    Set {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        enable: bool,
    },
}

#[derive(Debug, Subcommand)]
enum UsbPortCommand {
    Set {
        #[arg(value_name = "TARGET_OR_PORT", num_args = 0..=2)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum HardwareCommand {
    Available {
        #[arg(long)]
        scan: bool,
    },
    List,
    Recent,
    Path,
    Save {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        transport: SavedTransport,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        devd: Option<String>,
    },
    Forget {
        id: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum UsbPortTarget {
    Digital,
}

#[derive(Debug, Clone, ValueEnum)]
enum BoardTarget {
    Digital,
    Analog,
}

#[derive(Debug, Clone, ValueEnum)]
enum MonitorFormat {
    Human,
    Jsonl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum SavedTransport {
    Usb,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SavedHardware {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    transport: SavedTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    devd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen_unix_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HardwareRegistry {
    #[serde(default = "hardware_registry_schema_version")]
    schema_version: u8,
    #[serde(default)]
    hardware: Vec<SavedHardware>,
}

#[derive(Debug, Clone, Deserialize)]
struct CliLease {
    lease_id: String,
    heartbeat_interval_ms: u64,
}

impl Default for HardwareRegistry {
    fn default() -> Self {
        Self {
            schema_version: hardware_registry_schema_version(),
            hardware: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedUsbHardware {
    device: String,
    devd: String,
}

#[derive(Debug, Clone)]
enum ResolvedHardware {
    Usb(ResolvedUsbHardware),
    Http { url: String },
}

impl BoardTarget {
    fn kind(&self) -> TargetKind {
        match self {
            Self::Digital => TargetKind::DigitalEsp32s3,
            Self::Analog => TargetKind::AnalogStm32g431,
        }
    }
}

fn api_url(base: &str, path: &str) -> Result<Url, Box<dyn std::error::Error + Send + Sync>> {
    let base_url = Url::parse(base)?;
    let inherited_query = base_url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let mut url = base_url;
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    url.set_path(path);
    url.set_query((!query.is_empty()).then_some(query));
    let existing_keys = url
        .query_pairs()
        .map(|(key, _)| key.into_owned())
        .collect::<Vec<_>>();
    if !inherited_query.is_empty() {
        url.query_pairs_mut().extend_pairs(
            inherited_query
                .iter()
                .filter(|(key, _)| !existing_keys.contains(key))
                .map(|(key, value)| (&**key, &**value)),
        );
    }
    Ok(url)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let client = Client::new();
    let payload = match cli.command {
        Command::Hardware { command } => {
            handle_hardware_command(command, &client, &cli.devd).await?
        }
        Command::UsbPort {
            command: UsbPortCommand::Set { args },
        } => {
            let (target, port) = resolve_usb_port_set_args(args)?;
            let repo_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match target {
                UsbPortTarget::Digital => {
                    write_default_digital_usb_port(&repo_root, &port)?;
                    json!({"ok": true, "mcu": "digital", "default_usb_port": port})
                }
            }
        }
        Command::Discover { mdns, lan_scan } => {
            let scan = client
                .post(api_url(&cli.devd, "/api/v1/devices/scan")?)
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await?;
            json!({"mdns_requested": mdns, "lan_scan_requested": lan_scan, "devd": scan})
        }
        Command::Devices => {
            client
                .get(api_url(&cli.devd, "/api/v1/devices")?)
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await?
        }
        Command::Status {
            url,
            device,
            hardware,
        } => {
            ensure_one_status_selector(url.as_ref(), device.as_ref(), hardware.as_ref())?;
            if let Some(hardware_id) = hardware {
                match resolve_saved_hardware(&hardware_id, &cli.devd)? {
                    ResolvedHardware::Usb(resolved) => {
                        let lease =
                            create_cli_lease(&client, &resolved.devd, &resolved.device).await?;
                        let heartbeat = spawn_cli_lease_heartbeat(
                            client.clone(),
                            resolved.devd.clone(),
                            lease.clone(),
                        );
                        let mut url = api_url(&resolved.devd, "/api/v1/status")?;
                        url.query_pairs_mut()
                            .append_pair("device_id", &resolved.device)
                            .append_pair("lease_id", &lease.lease_id);
                        let status: Result<Value, Box<dyn std::error::Error + Send + Sync>> =
                            async {
                                Ok(client
                                    .get(url)
                                    .send()
                                    .await?
                                    .error_for_status()?
                                    .json::<Value>()
                                    .await?)
                            }
                            .await;
                        let _ = release_cli_lease(&client, &resolved.devd, &lease.lease_id).await;
                        heartbeat.abort();
                        let _ =
                            remember_connected_usb(&hardware_id, &resolved.device, &resolved.devd);
                        status?
                    }
                    ResolvedHardware::Http { url } => {
                        let status = client
                            .get(api_url(&url, "/api/v1/status")?)
                            .send()
                            .await?
                            .error_for_status()?
                            .json::<Value>()
                            .await?;
                        let _ = remember_connected_http(&hardware_id, &url);
                        status
                    }
                }
            } else if let Some(url) = url {
                let status = client
                    .get(api_url(&url, "/api/v1/status")?)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?;
                let id = generated_http_hardware_id(&url)?;
                let _ = remember_connected_http(&id, &url);
                status
            } else if let Some(device) = device {
                let lease = create_cli_lease(&client, &cli.devd, &device).await?;
                let heartbeat =
                    spawn_cli_lease_heartbeat(client.clone(), cli.devd.clone(), lease.clone());
                let mut url = api_url(&cli.devd, "/api/v1/status")?;
                url.query_pairs_mut()
                    .append_pair("device_id", &device)
                    .append_pair("lease_id", &lease.lease_id);
                let status: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
                    Ok(client
                        .get(url)
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?)
                }
                .await;
                let _ = release_cli_lease(&client, &cli.devd, &lease.lease_id).await;
                heartbeat.abort();
                let id = generated_usb_hardware_id(&device);
                let _ = remember_connected_usb(&id, &device, &cli.devd);
                status?
            } else {
                return Err("status requires --hardware, --device, or --url".into());
            }
        }
        Command::Flash {
            target,
            device,
            hardware,
            artifact,
            dry_run,
        } => {
            let resolved = resolve_usb_target(device, hardware, &cli.devd)?;
            let lease = create_cli_lease(&client, &resolved.devd, &resolved.device).await?;
            let heartbeat =
                spawn_cli_lease_heartbeat(client.clone(), resolved.devd.clone(), lease.clone());
            let result: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
                Ok(client
                    .post(api_url(
                        &resolved.devd,
                        &format!("/api/v1/devices/{}/flash", resolved.device),
                    )?)
                    .json(&json!({"target": target.kind(), "artifact_id": artifact, "lease_id": lease.lease_id, "dry_run": dry_run}))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?)
            }
            .await;
            let _ = release_cli_lease(&client, &resolved.devd, &lease.lease_id).await;
            heartbeat.abort();
            result?
        }
        Command::Reset {
            target,
            device,
            hardware,
            dry_run,
        } => {
            let resolved = resolve_usb_target(device, hardware, &cli.devd)?;
            let lease = create_cli_lease(&client, &resolved.devd, &resolved.device).await?;
            let heartbeat =
                spawn_cli_lease_heartbeat(client.clone(), resolved.devd.clone(), lease.clone());
            let result: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
                Ok(client
                    .post(api_url(
                        &resolved.devd,
                        &format!("/api/v1/devices/{}/reset", resolved.device),
                    )?)
                    .json(&json!({"target": target.kind(), "lease_id": lease.lease_id, "dry_run": dry_run}))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?)
            }
            .await;
            let _ = release_cli_lease(&client, &resolved.devd, &lease.lease_id).await;
            heartbeat.abort();
            result?
        }
        Command::Monitor {
            target: _,
            device,
            hardware,
            tail,
            format,
        } => {
            let resolved = resolve_usb_target(device, hardware, &cli.devd)?;
            run_monitor(&client, resolved, tail, format).await?
        }
        Command::Output { command } => match command {
            OutputCommand::Set {
                url,
                hardware,
                enable,
            } => {
                ensure_one_output_selector(url.as_ref(), hardware.as_ref())?;
                if let Some(hardware_id) = hardware {
                    match resolve_saved_hardware(&hardware_id, &cli.devd)? {
                        ResolvedHardware::Http { url } => {
                            client
                                .post(api_url(&url, "/api/v1/cc")?)
                                .json(&json!({"enable": enable}))
                                .send()
                                .await?
                                .error_for_status()?
                                .json::<Value>()
                                .await?
                        }
                        ResolvedHardware::Usb(resolved) => {
                            let lease =
                                create_cli_lease(&client, &resolved.devd, &resolved.device).await?;
                            let heartbeat = spawn_cli_lease_heartbeat(
                                client.clone(),
                                resolved.devd.clone(),
                                lease.clone(),
                            );
                            let mut output_url = api_url(&resolved.devd, "/api/v1/cc")?;
                            output_url
                                .query_pairs_mut()
                                .append_pair("device_id", &resolved.device)
                                .append_pair("lease_id", &lease.lease_id);
                            let result: Result<Value, Box<dyn std::error::Error + Send + Sync>> =
                                async {
                                    Ok(client
                                        .post(output_url)
                                        .json(&json!({"enable": enable}))
                                        .send()
                                        .await?
                                        .error_for_status()?
                                        .json::<Value>()
                                        .await?)
                                }
                                .await;
                            let _ =
                                release_cli_lease(&client, &resolved.devd, &lease.lease_id).await;
                            heartbeat.abort();
                            result?
                        }
                    }
                } else if let Some(url) = url {
                    client
                        .post(api_url(&url, "/api/v1/cc")?)
                        .json(&json!({"enable": enable}))
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                } else {
                    return Err("output set requires --hardware or --url".into());
                }
            }
        },
    };

    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

async fn create_cli_lease(
    client: &Client,
    devd: &str,
    device: &str,
) -> Result<CliLease, Box<dyn std::error::Error + Send + Sync>> {
    Ok(client
        .post(api_url(devd, "/api/v1/serial/lease")?)
        .json(&json!({"device_id": device}))
        .send()
        .await?
        .error_for_status()?
        .json::<CliLease>()
        .await?)
}

async fn release_cli_lease(
    client: &Client,
    devd: &str,
    lease_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = client
        .delete(api_url(devd, &format!("/api/v1/serial/lease/{lease_id}"))?)
        .send()
        .await?;
    Ok(())
}

fn spawn_cli_lease_heartbeat(
    client: Client,
    devd: String,
    lease: CliLease,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval_ms = (lease.heartbeat_interval_ms / 2).max(500);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            let Ok(url) = api_url(&devd, &format!("/api/v1/serial/lease/{}", lease.lease_id))
            else {
                break;
            };
            if client.post(url).send().await.is_err() {
                break;
            }
        }
    })
}

async fn run_monitor(
    client: &Client,
    resolved: ResolvedUsbHardware,
    tail: usize,
    format: MonitorFormat,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let lease = create_cli_lease(client, &resolved.devd, &resolved.device).await?;
    let _heartbeat =
        spawn_cli_lease_heartbeat(client.clone(), resolved.devd.clone(), lease.clone());
    let mut seen = HashSet::new();
    loop {
        let mut url = api_url(
            &resolved.devd,
            &format!(
                "/api/v1/devices/{}/session?logs_limit={tail}&trace_limit={}",
                resolved.device,
                tail * 2
            ),
        )?;
        url.query_pairs_mut()
            .append_pair("lease_id", &lease.lease_id);
        let session = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        print_session_delta(&session, &mut seen, &format)?;
        tokio::time::sleep(std::time::Duration::from_millis(1_000)).await;
    }
}

fn print_session_delta(
    session: &Value,
    seen: &mut HashSet<String>,
    format: &MonitorFormat,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for kind in ["logs", "trace"] {
        let Some(items) = session.get(kind).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| serde_json::to_string(item).unwrap_or_default());
            if !seen.insert(format!("{kind}:{id}")) {
                continue;
            }
            match format {
                MonitorFormat::Jsonl => println!(
                    "{}",
                    serde_json::to_string(&json!({"kind": kind, "item": item}))?
                ),
                MonitorFormat::Human => {
                    if kind == "logs" {
                        println!(
                            "{} [{}] {}: {}",
                            item.get("timestamp").and_then(Value::as_str).unwrap_or("-"),
                            item.get("level").and_then(Value::as_str).unwrap_or("info"),
                            item.get("target").and_then(Value::as_str).unwrap_or("devd"),
                            item.get("message").and_then(Value::as_str).unwrap_or("")
                        );
                    } else {
                        println!(
                            "{} [{}] {} {}",
                            item.get("timestamp").and_then(Value::as_str).unwrap_or("-"),
                            item.get("direction").and_then(Value::as_str).unwrap_or("?"),
                            item.get("summary")
                                .and_then(Value::as_str)
                                .unwrap_or("frame"),
                            serde_json::to_string(item.get("payload").unwrap_or(&Value::Null))?
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn resolve_usb_port_set_args(
    args: Vec<String>,
) -> Result<(UsbPortTarget, String), Box<dyn std::error::Error + Send + Sync>> {
    match args.as_slice() {
        [] => Ok((
            UsbPortTarget::Digital,
            choose_digital_usb_port_interactive()?,
        )),
        [single] if single == "digital" => Ok((
            UsbPortTarget::Digital,
            choose_digital_usb_port_interactive()?,
        )),
        [single] => Ok((UsbPortTarget::Digital, single.clone())),
        [target, port] if target == "digital" => Ok((UsbPortTarget::Digital, port.clone())),
        [target, _] => Err(format!("unsupported USB port target: {target}").into()),
        _ => Err("usb-port set accepts at most TARGET and PORT".into()),
    }
}

fn choose_digital_usb_port_interactive() -> io::Result<String> {
    let candidates = list_digital_usb_port_candidates();
    if candidates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No serial ports found. Connect the ESP32-S3 digital USB CDC device and retry.",
        ));
    }

    if candidates.len() == 1 {
        let candidate = &candidates[0];
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Use serial port {}?", candidate.display_name))
            .default(true)
            .interact()
            .map_err(io::Error::other)?;
        if confirmed {
            return Ok(candidate.port_path.clone());
        }
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "USB port selection cancelled",
        ));
    }

    let items = candidates
        .iter()
        .map(|candidate| {
            if candidate.recognized {
                format!("{} (recognized dev board)", candidate.display_name)
            } else {
                candidate.display_name.clone()
            }
        })
        .collect::<Vec<_>>();
    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select ESP32-S3 digital USB CDC serial port")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(io::Error::other)?
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::Interrupted, "USB port selection cancelled")
        })?;

    Ok(candidates[selected].port_path.clone())
}

async fn handle_hardware_command(
    command: HardwareCommand,
    client: &Client,
    devd: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    match command {
        HardwareCommand::Available { scan } => {
            let registry = read_hardware_registry(&path)?;
            let scan_result = if scan {
                Some(match api_url(devd, "/api/v1/devices/scan") {
                    Ok(url) => match client.post(url).send().await {
                        Ok(response) => match response.error_for_status() {
                            Ok(response) => match response.json::<Value>().await {
                                Ok(value) => json!({"ok": true, "response": value}),
                                Err(error) => devd_error_payload(error),
                            },
                            Err(error) => devd_error_payload(error),
                        },
                        Err(error) => devd_error_payload(error),
                    },
                    Err(error) => devd_error_payload(error),
                })
            } else {
                None
            };
            let devd_devices = match api_url(devd, "/api/v1/devices") {
                Ok(url) => match client.get(url).send().await {
                    Ok(response) => match response.error_for_status() {
                        Ok(response) => match response.json::<Value>().await {
                            Ok(value) => json!({"ok": true, "response": value}),
                            Err(error) => devd_error_payload(error),
                        },
                        Err(error) => devd_error_payload(error),
                    },
                    Err(error) => devd_error_payload(error),
                },
                Err(error) => devd_error_payload(error),
            };
            Ok(available_hardware_payload(
                path,
                devd,
                scan,
                scan_result,
                devd_devices,
                registry,
            ))
        }
        HardwareCommand::List => {
            let mut registry = read_hardware_registry(&path)?;
            sort_hardware(&mut registry.hardware);
            Ok(json!({"path": path, "hardware": registry.hardware}))
        }
        HardwareCommand::Recent => {
            let mut registry = read_hardware_registry(&path)?;
            sort_recent_hardware(&mut registry.hardware);
            Ok(json!({"path": path, "hardware": registry.hardware}))
        }
        HardwareCommand::Path => Ok(json!({"path": path})),
        HardwareCommand::Save {
            id,
            name,
            transport,
            device,
            url,
            devd,
        } => {
            validate_manual_hardware(&id, &transport, device.as_deref(), url.as_deref())?;
            let mut registry = read_hardware_registry(&path)?;
            let hardware = SavedHardware {
                id,
                name,
                transport,
                device,
                url,
                devd,
                last_seen_unix_seconds: Some(current_unix_seconds()),
            };
            let saved = upsert_hardware(&mut registry, hardware);
            write_hardware_registry(&path, &registry)?;
            Ok(json!({"path": path, "hardware": saved}))
        }
        HardwareCommand::Forget { id } => {
            let mut registry = read_hardware_registry(&path)?;
            let before = registry.hardware.len();
            registry.hardware.retain(|hardware| hardware.id != id);
            let removed = registry.hardware.len() != before;
            write_hardware_registry(&path, &registry)?;
            Ok(json!({"path": path, "id": id, "removed": removed}))
        }
    }
}

fn resolve_saved_hardware(
    id: &str,
    default_devd: &str,
) -> Result<ResolvedHardware, Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let registry = read_hardware_registry(&path)?;
    let hardware = registry
        .hardware
        .iter()
        .find(|hardware| hardware.id == id)
        .ok_or_else(|| format!("saved hardware not found: {id}"))?;

    match &hardware.transport {
        SavedTransport::Usb => {
            let device = hardware
                .device
                .clone()
                .ok_or_else(|| format!("saved USB hardware {id} is missing device"))?;
            Ok(ResolvedHardware::Usb(ResolvedUsbHardware {
                device,
                devd: hardware
                    .devd
                    .clone()
                    .unwrap_or_else(|| default_devd.to_string()),
            }))
        }
        SavedTransport::Http => {
            let url = hardware
                .url
                .clone()
                .ok_or_else(|| format!("saved HTTP hardware {id} is missing url"))?;
            Ok(ResolvedHardware::Http { url })
        }
    }
}

fn resolve_usb_target(
    device: Option<String>,
    hardware: Option<String>,
    default_devd: &str,
) -> Result<ResolvedUsbHardware, Box<dyn std::error::Error + Send + Sync>> {
    if device.is_some() && hardware.is_some() {
        return Err("command accepts only one of --hardware or --device".into());
    }
    if let Some(hardware_id) = hardware {
        match resolve_saved_hardware(&hardware_id, default_devd)? {
            ResolvedHardware::Usb(resolved) => Ok(resolved),
            ResolvedHardware::Http { .. } => Err(format!(
                "saved hardware {hardware_id} uses HTTP; this command requires USB/devd hardware"
            )
            .into()),
        }
    } else if let Some(device) = device {
        Ok(ResolvedUsbHardware {
            device,
            devd: default_devd.to_string(),
        })
    } else {
        Err("command requires --hardware or --device".into())
    }
}

fn ensure_one_status_selector(
    url: Option<&String>,
    device: Option<&String>,
    hardware: Option<&String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let count = [url.is_some(), device.is_some(), hardware.is_some()]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    match count {
        0 => Err("status requires --hardware, --device, or --url".into()),
        1 => Ok(()),
        _ => Err("status accepts only one of --hardware, --device, or --url".into()),
    }
}

fn ensure_one_output_selector(
    url: Option<&String>,
    hardware: Option<&String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match (url.is_some(), hardware.is_some()) {
        (true, true) => Err("output set accepts only one of --hardware or --url".into()),
        (false, false) => Err("output set requires --hardware or --url".into()),
        _ => Ok(()),
    }
}

fn validate_manual_hardware(
    id: &str,
    transport: &SavedTransport,
    device: Option<&str>,
    url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if id.trim().is_empty() {
        return Err("hardware id must not be empty".into());
    }
    match transport {
        SavedTransport::Usb if device.is_none() => {
            Err("USB hardware records require --device".into())
        }
        SavedTransport::Http if url.is_none() => Err("HTTP hardware records require --url".into()),
        SavedTransport::Http => {
            Url::parse(url.expect("checked above"))?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn remember_connected_usb(
    id: &str,
    device: &str,
    devd: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let mut registry = read_hardware_registry(&path)?;
    upsert_hardware(
        &mut registry,
        SavedHardware {
            id: id.to_string(),
            name: None,
            transport: SavedTransport::Usb,
            device: Some(device.to_string()),
            url: None,
            devd: Some(devd.to_string()),
            last_seen_unix_seconds: Some(current_unix_seconds()),
        },
    );
    write_hardware_registry(&path, &registry)
}

fn remember_connected_http(
    id: &str,
    url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let mut registry = read_hardware_registry(&path)?;
    upsert_hardware(
        &mut registry,
        SavedHardware {
            id: id.to_string(),
            name: None,
            transport: SavedTransport::Http,
            device: None,
            url: Some(url.to_string()),
            devd: None,
            last_seen_unix_seconds: Some(current_unix_seconds()),
        },
    );
    write_hardware_registry(&path, &registry)
}

fn upsert_hardware(registry: &mut HardwareRegistry, mut hardware: SavedHardware) -> SavedHardware {
    if let Some(existing) = registry
        .hardware
        .iter_mut()
        .find(|existing| existing.id == hardware.id)
    {
        if hardware.name.is_none() {
            hardware.name = existing.name.clone();
        }
        *existing = hardware.clone();
        sort_hardware(&mut registry.hardware);
        return hardware;
    }
    registry.hardware.push(hardware.clone());
    sort_hardware(&mut registry.hardware);
    hardware
}

fn sort_hardware(hardware: &mut [SavedHardware]) {
    hardware.sort_by(|left, right| {
        transport_rank(&left.transport)
            .cmp(&transport_rank(&right.transport))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn sort_recent_hardware(hardware: &mut [SavedHardware]) {
    hardware.sort_by(|left, right| {
        right
            .last_seen_unix_seconds
            .unwrap_or(0)
            .cmp(&left.last_seen_unix_seconds.unwrap_or(0))
            .then_with(|| transport_rank(&left.transport).cmp(&transport_rank(&right.transport)))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn available_hardware_payload(
    path: PathBuf,
    devd: &str,
    scan: bool,
    scan_result: Option<Value>,
    devd_devices: Value,
    mut registry: HardwareRegistry,
) -> Value {
    sort_hardware(&mut registry.hardware);
    let remembered_usb = registry
        .hardware
        .iter()
        .filter(|hardware| hardware.transport == SavedTransport::Usb)
        .cloned()
        .collect::<Vec<_>>();
    let remembered_http = registry
        .hardware
        .iter()
        .filter(|hardware| hardware.transport == SavedTransport::Http)
        .cloned()
        .collect::<Vec<_>>();

    json!({
        "path": path,
        "devd": devd,
        "scan_requested": scan,
        "scan": scan_result,
        "usb": {
            "devices": devd_devices,
            "remembered": remembered_usb,
        },
        "http_fallback": remembered_http,
    })
}

fn devd_error_payload(error: impl std::fmt::Display) -> Value {
    json!({"ok": false, "error": error.to_string()})
}

fn transport_rank(transport: &SavedTransport) -> u8 {
    match transport {
        SavedTransport::Usb => 0,
        SavedTransport::Http => 1,
    }
}

fn read_hardware_registry(
    path: &Path,
) -> Result<HardwareRegistry, Box<dyn std::error::Error + Send + Sync>> {
    if !path.exists() {
        return Ok(HardwareRegistry::default());
    }
    let content = fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(HardwareRegistry::default());
    }
    Ok(serde_json::from_str(&content)?)
}

fn write_hardware_registry(
    path: &Path,
    registry: &HardwareRegistry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(registry)?)?;
    Ok(())
}

fn hardware_registry_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    hardware_registry_path_from_values(
        env::consts::OS,
        env::var_os("LOADLYNX_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        env::var_os("APPDATA").map(PathBuf::from),
        env::var_os("USERPROFILE").map(PathBuf::from),
    )
    .ok_or_else(|| "cannot resolve user config directory; set LOADLYNX_HOME".into())
}

fn hardware_registry_path_from_values(
    os: &str,
    loadlynx_home: Option<PathBuf>,
    home: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    appdata: Option<PathBuf>,
    userprofile: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(loadlynx_home) = loadlynx_home.filter(|path| !path.as_os_str().is_empty()) {
        return Some(loadlynx_home.join("devices.json"));
    }

    match os {
        "macos" => home.map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("LoadLynx")
                .join("devices.json")
        }),
        "windows" => appdata
            .map(|appdata| appdata.join("LoadLynx").join("devices.json"))
            .or_else(|| {
                userprofile.map(|home| home.join(".config").join("loadlynx").join("devices.json"))
            }),
        _ => xdg_config_home
            .map(|xdg| xdg.join("loadlynx").join("devices.json"))
            .or_else(|| {
                home.map(|home| home.join(".config").join("loadlynx").join("devices.json"))
            }),
    }
}

fn generated_usb_hardware_id(device: &str) -> String {
    format!("usb-{}", sanitize_hardware_id(device))
}

fn generated_http_hardware_id(
    base_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = Url::parse(base_url)?;
    let mut id = url.host_str().unwrap_or("device").to_string();
    if let Some(port) = url.port() {
        id.push('-');
        id.push_str(&port.to_string());
    }
    Ok(format!("http-{}", sanitize_hardware_id(&id)))
}

fn sanitize_hardware_id(input: &str) -> String {
    let mut id = String::new();
    let mut last_was_dash = false;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            id.push('-');
            last_was_dash = true;
        }
    }
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "device".to_string()
    } else {
        id
    }
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hardware_registry_schema_version() -> u8 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_port_set_args_accept_port_only() {
        let (target, port) =
            resolve_usb_port_set_args(vec!["/dev/cu.usbmodem212101".to_string()]).unwrap();
        assert!(matches!(target, UsbPortTarget::Digital));
        assert_eq!(port, "/dev/cu.usbmodem212101");
    }

    #[test]
    fn usb_port_set_args_accept_digital_and_port() {
        let (target, port) = resolve_usb_port_set_args(vec![
            "digital".to_string(),
            "/dev/cu.usbmodem212101".to_string(),
        ])
        .unwrap();
        assert!(matches!(target, UsbPortTarget::Digital));
        assert_eq!(port, "/dev/cu.usbmodem212101");
    }

    #[test]
    fn usb_port_set_args_reject_unknown_target() {
        let err = resolve_usb_port_set_args(vec![
            "analog".to_string(),
            "/dev/cu.usbmodem212101".to_string(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("unsupported USB port target"));
    }

    #[test]
    fn flash_defaults_to_dry_run_and_accepts_no_dry_run() {
        let cli =
            Cli::try_parse_from(["loadlynx", "flash", "--device", "digital-1", "digital"]).unwrap();
        match cli.command {
            Command::Flash { dry_run, .. } => assert!(dry_run),
            _ => panic!("expected flash command"),
        }

        let cli = Cli::try_parse_from([
            "loadlynx",
            "flash",
            "--device",
            "digital-1",
            "--no-dry-run",
            "digital",
        ])
        .unwrap();
        match cli.command {
            Command::Flash { dry_run, .. } => assert!(!dry_run),
            _ => panic!("expected flash command"),
        }
    }

    #[test]
    fn reset_defaults_to_dry_run_and_accepts_no_dry_run() {
        let cli =
            Cli::try_parse_from(["loadlynx", "reset", "--device", "digital-1", "digital"]).unwrap();
        match cli.command {
            Command::Reset { dry_run, .. } => assert!(dry_run),
            _ => panic!("expected reset command"),
        }

        let cli = Cli::try_parse_from([
            "loadlynx",
            "reset",
            "--device",
            "digital-1",
            "--no-dry-run",
            "digital",
        ])
        .unwrap();
        match cli.command {
            Command::Reset { dry_run, .. } => assert!(!dry_run),
            _ => panic!("expected reset command"),
        }
    }

    #[test]
    fn monitor_rejects_lease_id_and_accepts_format() {
        assert!(
            Cli::try_parse_from([
                "loadlynx",
                "monitor",
                "--device",
                "digital-1",
                "--lease-id",
                "lease-1",
                "digital",
            ])
            .is_err()
        );

        let cli = Cli::try_parse_from([
            "loadlynx",
            "monitor",
            "--device",
            "digital-1",
            "--format",
            "jsonl",
            "digital",
        ])
        .unwrap();
        match cli.command {
            Command::Monitor { format, tail, .. } => {
                assert!(matches!(format, MonitorFormat::Jsonl));
                assert_eq!(tail, 200);
            }
            _ => panic!("expected monitor command"),
        }
    }

    #[test]
    fn hardware_registry_path_uses_user_config_locations() {
        let override_path = hardware_registry_path_from_values(
            "macos",
            Some(PathBuf::from("/tmp/loadlynx-home")),
            Some(PathBuf::from("/Users/alice")),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            override_path,
            PathBuf::from("/tmp/loadlynx-home").join("devices.json")
        );

        let macos_path = hardware_registry_path_from_values(
            "macos",
            None,
            Some(PathBuf::from("/Users/alice")),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            macos_path,
            PathBuf::from("/Users/alice")
                .join("Library")
                .join("Application Support")
                .join("LoadLynx")
                .join("devices.json")
        );

        let linux_path = hardware_registry_path_from_values(
            "linux",
            None,
            Some(PathBuf::from("/home/alice")),
            Some(PathBuf::from("/home/alice/.config-custom")),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            linux_path,
            PathBuf::from("/home/alice/.config-custom")
                .join("loadlynx")
                .join("devices.json")
        );

        let windows_path = hardware_registry_path_from_values(
            "windows",
            None,
            None,
            None,
            Some(PathBuf::from("C:/Users/Alice/AppData/Roaming")),
            None,
        )
        .unwrap();
        assert_eq!(
            windows_path,
            PathBuf::from("C:/Users/Alice/AppData/Roaming")
                .join("LoadLynx")
                .join("devices.json")
        );
    }

    #[test]
    fn hardware_registry_round_trips_and_sorts_usb_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devices.json");
        let mut registry = HardwareRegistry::default();
        upsert_hardware(
            &mut registry,
            SavedHardware {
                id: "http-bench".to_string(),
                name: None,
                transport: SavedTransport::Http,
                device: None,
                url: Some("http://loadlynx.local".to_string()),
                devd: None,
                last_seen_unix_seconds: Some(10),
            },
        );
        upsert_hardware(
            &mut registry,
            SavedHardware {
                id: "usb-bench".to_string(),
                name: Some("Bench".to_string()),
                transport: SavedTransport::Usb,
                device: Some("digital-1".to_string()),
                url: None,
                devd: Some("http://127.0.0.1:30180".to_string()),
                last_seen_unix_seconds: Some(20),
            },
        );
        write_hardware_registry(&path, &registry).unwrap();

        let reloaded = read_hardware_registry(&path).unwrap();
        assert_eq!(reloaded.hardware[0].id, "usb-bench");
        assert_eq!(reloaded.hardware[1].id, "http-bench");
    }

    #[test]
    fn hardware_commands_parse_saved_device_workflows() {
        let cli = Cli::try_parse_from([
            "loadlynx",
            "hardware",
            "save",
            "--id",
            "bench",
            "--transport",
            "usb",
            "--device",
            "digital-1",
        ])
        .unwrap();
        match cli.command {
            Command::Hardware {
                command:
                    HardwareCommand::Save {
                        id,
                        transport,
                        device,
                        ..
                    },
            } => {
                assert_eq!(id, "bench");
                assert_eq!(transport, SavedTransport::Usb);
                assert_eq!(device.as_deref(), Some("digital-1"));
            }
            _ => panic!("expected hardware save command"),
        }

        let cli =
            Cli::try_parse_from(["loadlynx", "status", "--hardware", "usb-digital-1"]).unwrap();
        match cli.command {
            Command::Status { hardware, .. } => {
                assert_eq!(hardware.as_deref(), Some("usb-digital-1"));
            }
            _ => panic!("expected status command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "hardware", "available", "--scan"]).unwrap();
        match cli.command {
            Command::Hardware {
                command: HardwareCommand::Available { scan },
            } => assert!(scan),
            _ => panic!("expected hardware available command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "hardware", "recent"]).unwrap();
        match cli.command {
            Command::Hardware {
                command: HardwareCommand::Recent,
            } => {}
            _ => panic!("expected hardware recent command"),
        }
    }

    #[test]
    fn generated_hardware_ids_are_stable() {
        assert_eq!(
            generated_usb_hardware_id("Mock LoadLynx/devd"),
            "usb-mock-loadlynx-devd"
        );
        assert_eq!(
            generated_http_hardware_id("http://loadlynx-1234.local:8080").unwrap(),
            "http-loadlynx-1234-local-8080"
        );
    }

    #[test]
    fn recent_hardware_sorts_by_last_seen_descending() {
        let mut hardware = vec![
            SavedHardware {
                id: "old-usb".to_string(),
                name: None,
                transport: SavedTransport::Usb,
                device: Some("old".to_string()),
                url: None,
                devd: None,
                last_seen_unix_seconds: Some(10),
            },
            SavedHardware {
                id: "new-http".to_string(),
                name: None,
                transport: SavedTransport::Http,
                device: None,
                url: Some("http://new.local".to_string()),
                devd: None,
                last_seen_unix_seconds: Some(30),
            },
            SavedHardware {
                id: "new-usb".to_string(),
                name: None,
                transport: SavedTransport::Usb,
                device: Some("new".to_string()),
                url: None,
                devd: None,
                last_seen_unix_seconds: Some(30),
            },
        ];

        sort_recent_hardware(&mut hardware);

        assert_eq!(
            hardware
                .iter()
                .map(|hardware| hardware.id.as_str())
                .collect::<Vec<_>>(),
            vec!["new-usb", "new-http", "old-usb"]
        );
    }

    #[test]
    fn available_hardware_payload_keeps_usb_and_http_fallback_separate() {
        let mut registry = HardwareRegistry::default();
        upsert_hardware(
            &mut registry,
            SavedHardware {
                id: "http-bench".to_string(),
                name: None,
                transport: SavedTransport::Http,
                device: None,
                url: Some("http://loadlynx.local".to_string()),
                devd: None,
                last_seen_unix_seconds: Some(10),
            },
        );
        upsert_hardware(
            &mut registry,
            SavedHardware {
                id: "usb-bench".to_string(),
                name: None,
                transport: SavedTransport::Usb,
                device: Some("digital-1".to_string()),
                url: None,
                devd: Some("http://127.0.0.1:30180".to_string()),
                last_seen_unix_seconds: Some(20),
            },
        );

        let payload = available_hardware_payload(
            PathBuf::from("/tmp/loadlynx/devices.json"),
            "http://127.0.0.1:30180",
            false,
            None,
            json!({"devices": [{"id": "digital-1"}]}),
            registry,
        );

        assert_eq!(
            payload
                .pointer("/usb/remembered/0/id")
                .and_then(Value::as_str),
            Some("usb-bench")
        );
        assert_eq!(
            payload
                .pointer("/http_fallback/0/id")
                .and_then(Value::as_str),
            Some("http-bench")
        );
        assert_eq!(payload.get("scan").unwrap(), &Value::Null);
    }

    #[test]
    fn selectors_reject_ambiguous_saved_hardware_inputs() {
        let status_err = ensure_one_status_selector(
            Some(&"http://loadlynx.local".to_string()),
            None,
            Some(&"bench".to_string()),
        )
        .unwrap_err();
        assert!(status_err.to_string().contains("status accepts only one"));

        let usb_err = resolve_usb_target(
            Some("digital-1".to_string()),
            Some("bench".to_string()),
            "http://127.0.0.1:30180",
        )
        .unwrap_err();
        assert!(usb_err.to_string().contains("only one"));

        let output_err = ensure_one_output_selector(
            Some(&"http://loadlynx.local".to_string()),
            Some(&"bench".to_string()),
        )
        .unwrap_err();
        assert!(output_err.to_string().contains("only one"));
    }
}
