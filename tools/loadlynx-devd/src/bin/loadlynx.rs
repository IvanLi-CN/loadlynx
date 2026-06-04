use chrono::Utc;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use loadlynx_devd::{
    FLASH_CONFIRMATION_TEXT, IpcHttpRequest, TargetKind, default_ipc_endpoint, ipc_http_request,
    list_digital_usb_port_candidates, write_default_digital_usb_port,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    env, fs, io,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Parser)]
#[command(name = "loadlynx")]
#[command(about = "LoadLynx LAN/USB/devd control CLI")]
struct Cli {
    #[arg(long, global = true, default_value_t = default_ipc_endpoint())]
    ipc: String,
    #[arg(long, global = true)]
    no_auto_start: bool,
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
        #[arg(long = "manifest-path")]
        manifest_path: Option<String>,
        #[arg(long = "no-dry-run", default_value_t = true, action = ArgAction::SetFalse)]
        dry_run: bool,
        #[arg(long = "confirm", alias = "confirm-phrase")]
        confirm: Option<String>,
        #[arg(long)]
        expected_identity_device_id: Option<String>,
        #[arg(long)]
        acknowledge_non_project_firmware: bool,
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
    Pd {
        #[command(subcommand)]
        command: PdCommand,
    },
    Wifi {
        #[command(subcommand)]
        command: WifiCommand,
    },
    Control {
        #[command(subcommand)]
        command: ControlCommand,
    },
    Preset {
        #[command(subcommand)]
        command: PresetCommand,
    },
    Calibration {
        #[command(subcommand)]
        command: CalibrationCommand,
    },
    SoftReset {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long, default_value = "manual")]
        reason: String,
    },
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
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
        #[arg(long = "target-i-ma")]
        target_i_ma: Option<u32>,
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        disable: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PdCommand {
    Set {
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long, value_enum)]
        mode: Option<PdModeArg>,
        #[arg(long = "object-pos")]
        object_pos: Option<u8>,
        #[arg(long = "target-mv")]
        target_mv: Option<u32>,
        #[arg(long = "i-req-ma")]
        i_req_ma: Option<u32>,
        #[arg(long = "allow-extended-voltage")]
        allow_extended_voltage: Option<bool>,
    },
}

#[derive(Debug, Subcommand)]
enum WifiCommand {
    Show {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
    },
    Set {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        ssid: String,
        #[arg(long)]
        psk: String,
        #[arg(long)]
        wait: bool,
        #[arg(long)]
        allow_insecure_lan_wifi: bool,
    },
    Clear {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        allow_insecure_lan_wifi: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ControlCommand {
    Get {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
    },
    Set {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        disable: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PresetCommand {
    List {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
    },
    Set {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        file: PathBuf,
    },
    Apply {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        preset_id: u8,
    },
}

#[derive(Debug, Subcommand)]
enum CalibrationCommand {
    Profile {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
    },
    Mode {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        kind: String,
    },
    Apply {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        file: PathBuf,
    },
    Commit {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        file: PathBuf,
    },
    Reset {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        kind: String,
    },
}

#[derive(Debug, Subcommand)]
enum DiagnosticsCommand {
    Export {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum BackupCommand {
    Export {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(long = "include", value_delimiter = ',')]
        include: Vec<String>,
    },
    Import {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(long = "include", value_delimiter = ',')]
        include: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        allow_insecure_lan_wifi: bool,
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
    Bind {
        #[arg(value_enum)]
        transport: SavedTransport,
        #[arg(long)]
        candidate: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        set_default: bool,
    },
    Default {
        #[command(subcommand)]
        command: HardwareDefaultCommand,
    },
    Use {
        id: String,
        #[arg(long, value_enum)]
        transport: SavedTransport,
    },
    List,
    Path,
    Forget {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum HardwareDefaultCommand {
    Show,
    Set { id: String },
    Clear,
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

#[derive(Debug, Clone, ValueEnum)]
enum PdModeArg {
    Fixed,
    Pps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum SavedTransport {
    Usb,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LegacySavedHardware {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SavedHardware {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_transport: Option<SavedTransport>,
    #[serde(default, skip_serializing_if = "SavedTransports::is_empty")]
    transports: SavedTransports,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen_unix_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct SavedTransports {
    #[serde(skip_serializing_if = "Option::is_none")]
    usb: Option<SavedUsbTransport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http: Option<SavedHttpTransport>,
}

impl SavedTransports {
    fn is_empty(&self) -> bool {
        self.usb.is_none() && self.http.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SavedUsbTransport {
    device: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    port_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    devd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SavedHttpTransport {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HardwareRegistry {
    #[serde(default = "hardware_registry_schema_version")]
    schema_version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_hardware_id: Option<String>,
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
            default_hardware_id: None,
            hardware: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedUsbHardware {
    hardware_id: String,
    device: String,
    devd: String,
    port_path: Option<String>,
    expected_identity_device_id: String,
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

async fn ensure_ipc_devd(
    endpoint: &str,
    auto_start: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if devd_health(endpoint).await.is_ok() {
        return Ok(());
    }
    if !auto_start {
        return Err(format!(
            "loadlynx-devd IPC is not available at {endpoint}; start `loadlynx-devd serve --endpoint {endpoint}` or omit --no-auto-start"
        )
        .into());
    }

    let mut child = spawn_ipc_devd_process(endpoint).await?;

    for _ in 0..100 {
        if devd_health(endpoint).await.is_ok() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            return Err(format!(
                "loadlynx-devd exited early while starting IPC at {endpoint}: {status}"
            )
            .into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    let _ = child.kill().await;
    Err(format!("loadlynx-devd IPC did not become ready at {endpoint}").into())
}

async fn devd_health(endpoint: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let value = request_devd_value(endpoint, reqwest::Method::GET, "/health", None).await?;
    if value.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err("loadlynx-devd health response was not ok".into())
    }
}

fn sibling_devd_binary() -> PathBuf {
    let exe_name = if cfg!(windows) {
        "loadlynx-devd.exe"
    } else {
        "loadlynx-devd"
    };
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(exe_name)))
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from(exe_name))
}

async fn spawn_ipc_devd_process(
    endpoint: &str,
) -> Result<tokio::process::Child, Box<dyn std::error::Error + Send + Sync>> {
    let devd_bin = sibling_devd_binary();
    if devd_bin.exists() {
        return TokioCommand::new(&devd_bin)
            .arg("serve")
            .arg("--endpoint")
            .arg(endpoint)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                format!(
                    "failed to auto-start {}: {error}",
                    devd_bin.to_string_lossy()
                )
                .into()
            });
    }

    let manifest = PathBuf::from("tools/loadlynx-devd/Cargo.toml");
    TokioCommand::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--bin")
        .arg("loadlynx-devd")
        .arg("--")
        .arg("serve")
        .arg("--endpoint")
        .arg(endpoint)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            format!(
                "failed to auto-start cargo fallback for {}: {error}",
                manifest.to_string_lossy()
            )
            .into()
        })
}

async fn request_devd_value(
    endpoint: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        let client = Client::new();
        return request_http_value(&client, endpoint, method, path, body).await;
    }

    let response = ipc_http_request(
        endpoint,
        IpcHttpRequest {
            method: method.as_str().to_string(),
            path: path.to_string(),
            body,
        },
    )
    .await?;
    if (200..300).contains(&response.status) {
        Ok(response.body)
    } else {
        Err(format!(
            "devd IPC request failed with HTTP {}: {}",
            response.status, response.body
        )
        .into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let json_output = cli.json;
    let devd = cli.ipc;
    for endpoint in initial_devd_endpoints(&cli.command, &devd) {
        ensure_ipc_devd(&endpoint, !cli.no_auto_start).await?;
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let payload_result: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
        let payload = match cli.command {
        Command::Hardware { command } => handle_hardware_command(command, &client, &devd).await?,
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
            let scan =
                request_devd_value(&devd, reqwest::Method::POST, "/api/v1/devices/scan", None)
                    .await?;
            json!({"mdns_requested": mdns, "lan_scan_requested": lan_scan, "devd": scan})
        }
        Command::Devices => {
            request_devd_value(&devd, reqwest::Method::GET, "/api/v1/devices", None).await?
        }
        Command::Status {
            url,
            device,
            hardware,
        } => {
            ensure_one_status_selector(url.as_ref(), device.as_ref(), hardware.as_ref())?;
            if let Some(device) = device {
                return Err(format!(
                    "temporary devd device id `{device}` cannot be used for status; bind it first with `loadlynx hardware bind usb --candidate {device}` and then use --hardware <hardware-id>"
                )
                .into());
            }
            if let Some(url) = url {
                let status = client
                    .get(api_url(&url, "/api/v1/status")?)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?;
                status
            } else if let Some(hardware_id) = hardware.or_else(|| Some("default".to_string())) {
                match resolve_saved_hardware(&hardware_id, &devd)? {
                    ResolvedHardware::Usb(resolved) => {
                        let status = request_devd_usb_value(
                            &client,
                            &resolved,
                            reqwest::Method::GET,
                            "/api/v1/status",
                            None,
                        )
                        .await?;
                        let _ =
                            mark_hardware_transport_used(&resolved.hardware_id, SavedTransport::Usb);
                        status
                    }
                    ResolvedHardware::Http { url } => {
                        let status = client
                            .get(api_url(&url, "/api/v1/status")?)
                            .send()
                            .await?
                            .error_for_status()?
                            .json::<Value>()
                            .await?;
                        let _ = mark_hardware_transport_used(&hardware_id, SavedTransport::Http);
                        status
                    }
                }
            } else {
                return Err("status requires a saved default hardware, --hardware, or --url".into());
            }
        }
        Command::Flash {
            target,
            device,
            hardware,
            artifact,
            manifest_path,
            dry_run,
            confirm,
            expected_identity_device_id,
            acknowledge_non_project_firmware,
        } => {
            let resolved = resolve_usb_target(device, hardware, &devd)?;
            if manifest_path.is_some() {
                select_device_artifact(&client, &resolved, manifest_path.clone(), artifact.clone())
                    .await?;
            }
            let confirmation_text = resolve_flash_confirmation_text(&target, dry_run, confirm)?;
            post_usb_operation_with_optional_lease(
                &client,
                &resolved,
                &format!("/api/v1/devices/{}/flash", resolved.device),
                json!({
                    "target": target.kind(),
                    "artifact_id": artifact,
                    "dry_run": dry_run,
                    "confirmation_phrase": confirmation_text,
                    "expected_identity_device_id": expected_identity_device_id,
                    "acknowledge_non_project_firmware": acknowledge_non_project_firmware,
                }),
                dry_run,
            )
            .await?
        }
        Command::Reset {
            target,
            device,
            hardware,
            dry_run,
        } => {
            let resolved = resolve_usb_target(device, hardware, &devd)?;
            post_usb_operation_with_optional_lease(
                &client,
                &resolved,
                &format!("/api/v1/devices/{}/reset", resolved.device),
                json!({"target": target.kind(), "dry_run": dry_run}),
                dry_run,
            )
            .await?
        }
        Command::Monitor {
            target: _,
            device,
            hardware,
            tail,
            format,
        } => {
            let resolved = resolve_usb_target(device, hardware, &devd)?;
            run_monitor(&client, resolved, tail, format).await?
        }
        Command::Output { command } => match command {
            OutputCommand::Set {
                url,
                hardware,
                target_i_ma,
                enable,
                disable,
            } => {
                let enable = resolve_output_enable(enable, disable)?;
                ensure_one_output_selector(url.as_ref(), hardware.as_ref())?;
                let body = output_set_body(enable, target_i_ma);
                if let Some(hardware_id) = hardware {
                    match resolve_saved_hardware(&hardware_id, &devd)? {
                        ResolvedHardware::Http { url } => {
                            client
                                .post(api_url(&url, "/api/v1/cc")?)
                                .json(&body)
                                .send()
                                .await?
                                .error_for_status()?
                                .json::<Value>()
                                .await?
                        }
                        ResolvedHardware::Usb(resolved) => {
                            request_devd_usb_value(
                                &client,
                                &resolved,
                                reqwest::Method::POST,
                                "/api/v1/cc",
                                Some(body.clone()),
                            )
                            .await?
                        }
                    }
                } else if let Some(url) = url {
                    client
                        .post(api_url(&url, "/api/v1/cc")?)
                        .json(&body)
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                } else {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector {
                            url: None,
                            device: None,
                            hardware: None,
                        },
                        reqwest::Method::POST,
                        "/api/v1/cc",
                        Some(body),
                        false,
                    )
                    .await?
                }
            }
        },
        Command::Pd { command } => match command {
            PdCommand::Set {
                device,
                hardware,
                mode,
                object_pos,
                target_mv,
                i_req_ma,
                allow_extended_voltage,
            } => {
                let resolved = resolve_usb_target(device, hardware, &devd)?;
                let mut body = serde_json::Map::new();
                if let Some(mode) = mode {
                    body.insert(
                        "mode".to_string(),
                        Value::String(
                            match mode {
                                PdModeArg::Fixed => "fixed",
                                PdModeArg::Pps => "pps",
                            }
                            .to_string(),
                        ),
                    );
                }
                if let Some(object_pos) = object_pos {
                    body.insert("object_pos".to_string(), json!(object_pos));
                }
                if let Some(target_mv) = target_mv {
                    body.insert("target_mv".to_string(), json!(target_mv));
                }
                if let Some(i_req_ma) = i_req_ma {
                    body.insert("i_req_ma".to_string(), json!(i_req_ma));
                }
                if let Some(allow_extended_voltage) = allow_extended_voltage {
                    body.insert(
                        "allow_extended_voltage".to_string(),
                        json!(allow_extended_voltage),
                    );
                }
                request_devd_usb_value(
                    &client,
                    &resolved,
                    reqwest::Method::POST,
                    "/api/v1/pd",
                    Some(Value::Object(body)),
                )
                .await?
            }
        },
        Command::Wifi { command } => match command {
            WifiCommand::Show {
                url,
                device,
                hardware,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::GET,
                    "/api/v1/wifi",
                    None,
                    false,
                )
                .await?
            }
            WifiCommand::Set {
                url,
                device,
                hardware,
                ssid,
                psk,
                wait,
                allow_insecure_lan_wifi,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/wifi",
                    Some(json!({"ssid": ssid, "psk": psk, "wait": wait})),
                    allow_insecure_lan_wifi,
                )
                .await?
            }
            WifiCommand::Clear {
                url,
                device,
                hardware,
                allow_insecure_lan_wifi,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::DELETE,
                    "/api/v1/wifi",
                    None,
                    allow_insecure_lan_wifi,
                )
                .await?
            }
        },
        Command::Control { command } => match command {
            ControlCommand::Get {
                url,
                device,
                hardware,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::GET,
                    "/api/v1/control",
                    None,
                    false,
                )
                .await?
            }
            ControlCommand::Set {
                url,
                device,
                hardware,
                enable,
                disable,
            } => {
                let output_enabled = resolve_output_enable(enable, disable)?;
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/control",
                    Some(json!({"output_enabled": output_enabled})),
                    false,
                )
                .await?
            }
        },
        Command::Preset { command } => match command {
            PresetCommand::List {
                url,
                device,
                hardware,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::GET,
                    "/api/v1/presets",
                    None,
                    false,
                )
                .await?
            }
            PresetCommand::Set {
                url,
                device,
                hardware,
                file,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/presets",
                    Some(read_json_file(&file)?),
                    false,
                )
                .await?
            }
            PresetCommand::Apply {
                url,
                device,
                hardware,
                preset_id,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/presets/apply",
                    Some(json!({"preset_id": preset_id})),
                    false,
                )
                .await?
            }
        },
        Command::Calibration { command } => match command {
            CalibrationCommand::Profile {
                url,
                device,
                hardware,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::GET,
                    "/api/v1/calibration/profile",
                    None,
                    false,
                )
                .await?
            }
            CalibrationCommand::Mode {
                url,
                device,
                hardware,
                kind,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/calibration/mode",
                    Some(json!({"kind": kind})),
                    false,
                )
                .await?
            }
            CalibrationCommand::Apply {
                url,
                device,
                hardware,
                file,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/calibration/apply",
                    Some(read_json_file(&file)?),
                    false,
                )
                .await?
            }
            CalibrationCommand::Commit {
                url,
                device,
                hardware,
                file,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/calibration/commit",
                    Some(read_json_file(&file)?),
                    false,
                )
                .await?
            }
            CalibrationCommand::Reset {
                url,
                device,
                hardware,
                kind,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::POST,
                    "/api/v1/calibration/reset",
                    Some(json!({"kind": kind})),
                    false,
                )
                .await?
            }
        },
        Command::SoftReset {
            url,
            device,
            hardware,
            reason,
        } => {
            request_api_value(
                &client,
                &devd,
                ApiSelector {
                    url,
                    device,
                    hardware,
                },
                reqwest::Method::POST,
                "/api/v1/soft-reset",
                Some(json!({"reason": reason})),
                false,
            )
            .await?
        }
        Command::Diagnostics { command } => match command {
            DiagnosticsCommand::Export {
                url,
                device,
                hardware,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    reqwest::Method::GET,
                    "/api/v1/diagnostics/export",
                    None,
                    false,
                )
                .await?
            }
        },
        Command::Backup { command } => match command {
            BackupCommand::Export {
                url,
                device,
                hardware,
                file,
                include,
            } => {
                handle_backup_export(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    &file,
                    &include,
                )
                .await?
            }
            BackupCommand::Import {
                url,
                device,
                hardware,
                file,
                include,
                dry_run,
                allow_insecure_lan_wifi,
            } => {
                handle_backup_import(
                    &client,
                    &devd,
                    ApiSelector {
                        url,
                        device,
                        hardware,
                    },
                    &file,
                    &include,
                    dry_run,
                    allow_insecure_lan_wifi,
                )
                .await?
            }
        },
        };
        Ok(payload)
    }
    .await;

    let payload = match payload_result {
        Ok(payload) => payload,
        Err(error) => {
            print_cli_error(&*error, json_output)?;
            std::process::exit(1);
        }
    };

    if payload
        .get("__loadlynx_cli_already_printed")
        .and_then(Value::as_bool)
        != Some(true)
    {
        print_cli_payload(&payload, json_output)?;
    }
    Ok(())
}

async fn select_device_artifact(
    _client: &Client,
    resolved: &ResolvedUsbHardware,
    manifest_path: Option<String>,
    artifact_id: Option<String>,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    request_devd_value(
        &resolved.devd,
        reqwest::Method::POST,
        &format!("/api/v1/devices/{}/artifact", resolved.device),
        Some(json!({"manifest_path": manifest_path, "artifact_id": artifact_id})),
    )
    .await
}

fn initial_devd_endpoints(command: &Command, default_devd: &str) -> Vec<String> {
    let endpoints = match command {
        Command::Discover { .. } | Command::Devices => vec![default_devd.to_string()],
        Command::Status {
            url,
            device,
            hardware,
        } => selector_devd_endpoint(
            url.as_ref(),
            device.as_ref(),
            hardware.as_ref(),
            default_devd,
        )
        .into_iter()
        .collect(),
        Command::Flash {
            device, hardware, ..
        }
        | Command::Reset {
            device, hardware, ..
        }
        | Command::Monitor {
            device, hardware, ..
        }
        | Command::Pd {
            command: PdCommand::Set {
                device, hardware, ..
            },
        } => usb_target_devd_endpoint(device.as_ref(), hardware.as_ref(), default_devd)
            .into_iter()
            .collect(),
        Command::Output {
            command: OutputCommand::Set { url, hardware, .. },
        } => selector_devd_endpoint(url.as_ref(), None, hardware.as_ref(), default_devd)
            .into_iter()
            .collect(),
        Command::Wifi { command } => match command {
            WifiCommand::Show {
                url,
                device,
                hardware,
            }
            | WifiCommand::Set {
                url,
                device,
                hardware,
                ..
            }
            | WifiCommand::Clear {
                url,
                device,
                hardware,
                ..
            } => selector_devd_endpoint(
                url.as_ref(),
                device.as_ref(),
                hardware.as_ref(),
                default_devd,
            )
            .into_iter()
            .collect(),
        },
        Command::Control { command } => match command {
            ControlCommand::Get {
                url,
                device,
                hardware,
            }
            | ControlCommand::Set {
                url,
                device,
                hardware,
                ..
            } => selector_devd_endpoint(
                url.as_ref(),
                device.as_ref(),
                hardware.as_ref(),
                default_devd,
            )
            .into_iter()
            .collect(),
        },
        Command::Preset { command } => match command {
            PresetCommand::List {
                url,
                device,
                hardware,
            }
            | PresetCommand::Set {
                url,
                device,
                hardware,
                ..
            }
            | PresetCommand::Apply {
                url,
                device,
                hardware,
                ..
            } => selector_devd_endpoint(
                url.as_ref(),
                device.as_ref(),
                hardware.as_ref(),
                default_devd,
            )
            .into_iter()
            .collect(),
        },
        Command::Calibration { command } => match command {
            CalibrationCommand::Profile {
                url,
                device,
                hardware,
            }
            | CalibrationCommand::Mode {
                url,
                device,
                hardware,
                ..
            }
            | CalibrationCommand::Apply {
                url,
                device,
                hardware,
                ..
            }
            | CalibrationCommand::Commit {
                url,
                device,
                hardware,
                ..
            }
            | CalibrationCommand::Reset {
                url,
                device,
                hardware,
                ..
            } => selector_devd_endpoint(
                url.as_ref(),
                device.as_ref(),
                hardware.as_ref(),
                default_devd,
            )
            .into_iter()
            .collect(),
        },
        Command::SoftReset {
            url,
            device,
            hardware,
            ..
        }
        | Command::Diagnostics {
            command:
                DiagnosticsCommand::Export {
                    url,
                    device,
                    hardware,
                },
        } => selector_devd_endpoint(
            url.as_ref(),
            device.as_ref(),
            hardware.as_ref(),
            default_devd,
        )
        .into_iter()
        .collect(),
        Command::Backup { command } => match command {
            BackupCommand::Export {
                url,
                device,
                hardware,
                ..
            }
            | BackupCommand::Import {
                url,
                device,
                hardware,
                ..
            } => selector_devd_endpoint(
                url.as_ref(),
                device.as_ref(),
                hardware.as_ref(),
                default_devd,
            )
            .into_iter()
            .collect(),
        },
        Command::UsbPort { .. } => Vec::new(),
        Command::Hardware {
            command: HardwareCommand::Available { scan: true },
        } => vec![default_devd.to_string()],
        Command::Hardware {
            command:
                HardwareCommand::Bind {
                    transport: SavedTransport::Usb,
                    ..
                },
        } => vec![default_devd.to_string()],
        Command::Hardware { .. } => Vec::new(),
    };

    let mut seen = HashSet::new();
    endpoints
        .into_iter()
        .filter(|endpoint| seen.insert(endpoint.clone()))
        .collect()
}

fn selector_devd_endpoint(
    url: Option<&String>,
    device: Option<&String>,
    hardware: Option<&String>,
    default_devd: &str,
) -> Option<String> {
    if url.is_some() {
        return None;
    }
    if device.is_some() {
        return Some(default_devd.to_string());
    }
    let resolved = hardware
        .and_then(|id| resolve_saved_hardware(id, default_devd).ok())
        .or_else(|| resolve_saved_hardware("default", default_devd).ok());
    resolved.and_then(|resolved| match resolved {
        ResolvedHardware::Usb(resolved) => Some(resolved.devd),
        ResolvedHardware::Http { .. } => None,
    })
}

fn usb_target_devd_endpoint(
    device: Option<&String>,
    hardware: Option<&String>,
    default_devd: &str,
) -> Option<String> {
    if device.is_some() {
        return Some(default_devd.to_string());
    }
    let resolved = hardware
        .and_then(|id| resolve_saved_hardware(id, default_devd).ok())
        .or_else(|| resolve_saved_hardware("default", default_devd).ok());
    resolved.and_then(|resolved| match resolved {
        ResolvedHardware::Usb(resolved) => Some(resolved.devd),
        ResolvedHardware::Http { .. } => None,
    })
}

fn output_set_body(enable: bool, target_i_ma: Option<u32>) -> Value {
    let mut body = serde_json::Map::new();
    body.insert("enable".to_string(), json!(enable));
    if let Some(target_i_ma) = target_i_ma {
        body.insert("target_i_ma".to_string(), json!(target_i_ma));
    }
    Value::Object(body)
}

fn resolve_flash_confirmation_text(
    target: &BoardTarget,
    dry_run: bool,
    provided: Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    if dry_run || !matches!(target, BoardTarget::Digital) {
        return Ok(provided);
    }
    if provided.is_some() {
        return Ok(provided);
    }
    eprintln!("Real digital firmware flash is high risk.");
    eprintln!("Type `{FLASH_CONFIRMATION_TEXT}` to continue.");
    let typed: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Confirmation")
        .allow_empty(false)
        .interact_text()?;
    Ok(Some(typed))
}

fn print_cli_payload(
    payload: &Value,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let payload = redact_cli_sensitive(payload);
    if json_output {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{}", render_human_payload(&payload)?);
    }
    Ok(())
}

fn print_cli_error(
    error: &(dyn std::error::Error + Send + Sync),
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message = error.to_string();
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": false,
                "error_code": classify_cli_error_code(&message),
                "error": message,
            }))?
        );
    } else {
        eprintln!("Error: {message}");
    }
    Ok(())
}

fn classify_cli_error_code(message: &str) -> &'static str {
    if message.contains("default hardware is not set") {
        "default_hardware_not_set"
    } else if message.contains("saved hardware not found") {
        "hardware_not_found"
    } else if message.contains("is not a stable LoadLynx hardware id") {
        "unstable_hardware_identity"
    } else if message.contains("identity_confirmation_mismatch") {
        "identity_confirmation_mismatch"
    } else if message.contains("device_not_found") {
        "device_not_found"
    } else if message.contains("target_selector_not_cached") {
        "target_selector_not_cached"
    } else if message.contains("bind it first") {
        "hardware_not_bound"
    } else {
        "command_failed"
    }
}

fn render_human_payload(payload: &Value) -> Result<String, serde_json::Error> {
    let view = payload.get("wifi").unwrap_or(payload);
    if view.get("state").is_some() && view.get("source").is_some() && view.get("ssid").is_some() {
        return Ok(format!(
            "WiFi: {} ssid={} source={} ip={}{}",
            str_field(view, "state").unwrap_or("unknown"),
            str_field(view, "ssid").unwrap_or("-"),
            str_field(view, "source").unwrap_or("-"),
            str_field(view, "ip").unwrap_or("-"),
            str_field(view, "last_error")
                .map(|error| format!(" error={error}"))
                .unwrap_or_default()
        ));
    }

    if payload.get("output_enabled").is_some() && payload.get("active_preset_id").is_some() {
        return Ok(format!(
            "Control: output={} active_preset={} uv_latched={}",
            bool_field(payload, "output_enabled").unwrap_or(false),
            payload
                .get("active_preset_id")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            bool_field(payload, "uv_latched").unwrap_or(false)
        ));
    }

    if payload.get("dry_run").and_then(Value::as_bool) == Some(true)
        && let Some(sections) = payload.get("would_restore").and_then(Value::as_array)
    {
        let section_list = sections
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        let mut out = format!(
            "Backup dry-run: would restore {}",
            if section_list.is_empty() {
                "nothing".to_string()
            } else {
                section_list
            }
        );
        append_backup_warnings(&mut out, payload);
        return Ok(out);
    }

    if let Some(restored) = payload.get("restored").and_then(Value::as_array) {
        let section_list = restored
            .iter()
            .map(|entry| {
                let section = str_field(entry, "section").unwrap_or("unknown");
                let status = if bool_field(entry, "ok").unwrap_or(false) {
                    "ok"
                } else {
                    "failed"
                };
                format!("{section}={status}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let mut out = format!(
            "Backup restore: {}",
            if section_list.is_empty() {
                "nothing restored".to_string()
            } else {
                section_list
            }
        );
        append_backup_warnings(&mut out, payload);
        return Ok(out);
    }

    if let Some(presets) = payload.get("presets").and_then(Value::as_array) {
        let mut out = String::from("Presets:");
        for preset in presets {
            out.push_str("\n  ");
            out.push_str(&render_preset_line(preset));
        }
        return Ok(out);
    }

    if payload.get("accepted").and_then(Value::as_bool) == Some(true) {
        return Ok(format!(
            "Accepted: {}",
            str_field(payload, "reason").unwrap_or("request")
        ));
    }

    if payload.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok("OK".to_string());
    }

    if let Some(devices) = payload.get("devices").and_then(Value::as_array) {
        return Ok(format!("Devices: {} discovered", devices.len()));
    }

    serde_json::to_string_pretty(payload)
}

fn append_backup_warnings(out: &mut String, payload: &Value) {
    let Some(warnings) = payload.get("warnings").and_then(Value::as_array) else {
        return;
    };
    if warnings.is_empty() {
        return;
    }
    let text = warnings
        .iter()
        .map(|warning| {
            warning
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| warning.to_string())
        })
        .collect::<Vec<_>>()
        .join("; ");
    if !text.is_empty() {
        out.push_str("\nWarnings: ");
        out.push_str(&text);
    }
}

fn render_preset_line(preset: &Value) -> String {
    format!(
        "#{:<2} mode={} i={}mA v={}mV p={}mW",
        preset
            .get("preset_id")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        str_field(preset, "mode").unwrap_or("-"),
        preset
            .get("target_i_ma")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        preset
            .get("target_v_mv")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        preset
            .get("target_p_mw")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
    )
}

fn str_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(|value| match value {
        Value::String(s) => Some(s.as_str()),
        Value::Null => None,
        _ => None,
    })
}

fn bool_field(value: &Value, field: &str) -> Option<bool> {
    value.get(field).and_then(Value::as_bool)
}

fn redact_cli_sensitive(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let key_lc = key.to_ascii_lowercase();
                    if matches!(
                        key_lc.as_str(),
                        "psk" | "password" | "passphrase" | "secret" | "token"
                    ) {
                        (key.clone(), Value::String("<redacted>".to_string()))
                    } else {
                        (key.clone(), redact_cli_sensitive(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_cli_sensitive).collect()),
        _ => value.clone(),
    }
}

#[derive(Debug, Clone)]
struct ApiSelector {
    url: Option<String>,
    device: Option<String>,
    hardware: Option<String>,
}

fn read_json_file(path: &Path) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[derive(Debug, Clone, Copy)]
struct BackupSelection {
    presets: bool,
    calibration: bool,
    wifi: bool,
    pd: bool,
}

impl BackupSelection {
    fn all() -> Self {
        Self {
            presets: true,
            calibration: true,
            wifi: true,
            pd: true,
        }
    }

    fn selected_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.presets {
            names.push("presets");
        }
        if self.calibration {
            names.push("calibration");
        }
        if self.wifi {
            names.push("settings.wifi");
        }
        if self.pd {
            names.push("settings.pd");
        }
        names
    }
}

fn parse_backup_selection(
    include: &[String],
) -> Result<BackupSelection, Box<dyn std::error::Error + Send + Sync>> {
    if include.is_empty() {
        return Ok(BackupSelection::all());
    }
    let mut selection = BackupSelection {
        presets: false,
        calibration: false,
        wifi: false,
        pd: false,
    };
    for raw in include {
        for item in raw
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            match item {
                "all" => selection = BackupSelection::all(),
                "presets" => selection.presets = true,
                "calibration" => selection.calibration = true,
                "settings" => {
                    selection.wifi = true;
                    selection.pd = true;
                }
                "settings.wifi" | "wifi" => selection.wifi = true,
                "settings.pd" | "pd" => selection.pd = true,
                other => return Err(format!("unsupported backup include: {other}").into()),
            }
        }
    }
    Ok(selection)
}

async fn handle_backup_export(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    file: &Path,
    include: &[String],
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let selection = parse_backup_selection(include)?;
    let mut sections = serde_json::Map::new();
    let mut warnings = Vec::<Value>::new();

    if selection.presets {
        let presets = request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::GET,
            "/api/v1/presets",
            None,
            false,
        )
        .await?;
        let mut section = serde_json::Map::new();
        if let Some(value) = presets.get("presets").cloned() {
            section.insert("presets".to_string(), value);
        } else {
            section.insert("presets".to_string(), Value::Array(Vec::new()));
        }
        match request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::GET,
            "/api/v1/control",
            None,
            false,
        )
        .await
        {
            Ok(control) => {
                if let Some(active) = control.get("active_preset_id").cloned() {
                    section.insert("active_preset_id".to_string(), active);
                }
            }
            Err(err) => warnings.push(json!({
                "section": "presets",
                "message": format!("active_preset_id unavailable: {err}")
            })),
        }
        sections.insert("presets".to_string(), Value::Object(section));
    }

    if selection.calibration {
        sections.insert(
            "calibration".to_string(),
            request_api_value(
                client,
                default_devd,
                selector.clone(),
                reqwest::Method::GET,
                "/api/v1/calibration/profile",
                None,
                false,
            )
            .await?,
        );
    }

    let mut settings = serde_json::Map::new();
    if selection.wifi {
        settings.insert(
            "wifi".to_string(),
            request_api_value(
                client,
                default_devd,
                selector.clone(),
                reqwest::Method::GET,
                "/api/v1/wifi/credentials",
                None,
                false,
            )
            .await?,
        );
    }
    if selection.pd {
        let pd = request_api_value(
            client,
            default_devd,
            selector,
            reqwest::Method::GET,
            "/api/v1/pd",
            None,
            false,
        )
        .await?;
        settings.insert(
            "pd".to_string(),
            json!({
                "saved": pd.get("saved").cloned().unwrap_or(Value::Null),
                "allow_extended_voltage": pd
                    .get("allow_extended_voltage")
                    .cloned()
                    .unwrap_or(Value::Bool(false)),
            }),
        );
    }
    if !settings.is_empty() {
        sections.insert("settings".to_string(), Value::Object(settings));
    }

    let mut backup = json!({
        "kind": "loadlynx.backup",
        "schema_version": 1,
        "created_at": Utc::now().to_rfc3339(),
        "selected_sections": selection.selected_names(),
        "sections": Value::Object(sections),
    });
    if !warnings.is_empty()
        && let Some(object) = backup.as_object_mut()
    {
        object.insert("warnings".to_string(), Value::Array(warnings));
    }

    write_backup_file(file, &backup)?;
    if file == Path::new("-") {
        Ok(json!({"__loadlynx_cli_already_printed": true}))
    } else {
        Ok(json!({
            "ok": true,
            "file": file.display().to_string(),
            "sections": selection.selected_names(),
            "contains_plaintext_wifi_psk": selection.wifi
        }))
    }
}

async fn handle_backup_import(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    file: &Path,
    include: &[String],
    dry_run: bool,
    allow_insecure_lan_wifi: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let selection = parse_backup_selection(include)?;
    let backup = read_backup_file(file)?;
    validate_backup_envelope(&backup)?;
    let warnings = backup_unknown_section_warnings(&backup);
    let restore_sections = restorable_backup_sections(&backup, selection);
    if dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "would_restore": restore_sections,
            "warnings": warnings,
        }));
    }

    preflight_backup_restore(
        default_devd,
        &selector,
        &backup,
        selection,
        allow_insecure_lan_wifi,
    )?;
    disable_output_for_restore(client, default_devd, selector.clone()).await?;
    settle_after_restore_write().await;
    let mut results = Vec::<Value>::new();

    if selection.presets && backup.pointer("/sections/presets").is_some() {
        restore_presets(
            client,
            default_devd,
            selector.clone(),
            &backup,
            &mut results,
        )
        .await?;
    }
    if selection.calibration && backup.pointer("/sections/calibration").is_some() {
        restore_calibration(
            client,
            default_devd,
            selector.clone(),
            &backup,
            &mut results,
        )
        .await?;
    }
    if selection.pd && backup.pointer("/sections/settings/pd").is_some() {
        restore_pd(
            client,
            default_devd,
            selector.clone(),
            &backup,
            &mut results,
        )
        .await?;
    }
    if selection.wifi && backup.pointer("/sections/settings/wifi").is_some() {
        restore_wifi(
            client,
            default_devd,
            selector,
            &backup,
            &mut results,
            allow_insecure_lan_wifi,
        )
        .await?;
    }

    Ok(json!({
        "ok": true,
        "dry_run": false,
        "safety": {"output_disabled": true},
        "restored": results,
        "warnings": warnings,
    }))
}

fn preflight_backup_restore(
    default_devd: &str,
    selector: &ApiSelector,
    backup: &Value,
    selection: BackupSelection,
    allow_insecure_lan_wifi: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_one_api_selector(
        selector.url.as_ref(),
        selector.device.as_ref(),
        selector.hardware.as_ref(),
    )?;

    if !selection.wifi || backup.pointer("/sections/settings/wifi").is_none() {
        return Ok(());
    }

    let is_lan_restore = if selector.url.is_some() {
        true
    } else if let Some(hardware_id) = selector.hardware.as_ref() {
        matches!(
            resolve_saved_hardware(hardware_id, default_devd)?,
            ResolvedHardware::Http { .. }
        )
    } else {
        false
    };

    if is_lan_restore && !allow_insecure_lan_wifi {
        return Err("LAN WiFi writes require --allow-insecure-lan-wifi".into());
    }

    Ok(())
}

fn read_backup_file(path: &Path) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    if path == Path::new("-") {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        Ok(serde_json::from_str(&input)?)
    } else {
        read_json_file(path)
    }
}

fn write_backup_file(
    path: &Path,
    backup: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bytes = serde_json::to_vec_pretty(backup)?;
    if path == Path::new("-") {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&bytes)?;
        stdout.write_all(b"\n")?;
    } else {
        write_private_backup_file(path, &bytes)?;
    }
    Ok(())
}

#[cfg(unix)]
fn write_private_backup_file(
    path: &Path,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private_backup_file(
    path: &Path,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fs::write(path, bytes)?;
    Ok(())
}

fn validate_backup_envelope(
    backup: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if backup.get("kind").and_then(Value::as_str) != Some("loadlynx.backup") {
        return Err("backup kind must be loadlynx.backup".into());
    }
    if backup.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported backup schema_version".into());
    }
    if !backup.get("sections").is_some_and(Value::is_object) {
        return Err("backup sections must be an object".into());
    }
    Ok(())
}

fn backup_unknown_section_warnings(backup: &Value) -> Vec<Value> {
    let mut warnings = Vec::new();
    let Some(sections) = backup.get("sections").and_then(Value::as_object) else {
        return warnings;
    };
    for key in sections.keys() {
        if key != "presets" && key != "calibration" && key != "settings" {
            warnings.push(json!({"section": key, "message": "unknown section ignored"}));
        }
    }
    if let Some(settings) = sections.get("settings").and_then(Value::as_object) {
        for key in settings.keys() {
            if key != "wifi" && key != "pd" {
                warnings.push(json!({
                    "section": format!("settings.{key}"),
                    "message": "unknown settings section ignored"
                }));
            }
        }
    }
    warnings
}

fn restorable_backup_sections(backup: &Value, selection: BackupSelection) -> Vec<&'static str> {
    let mut sections = Vec::new();
    if selection.presets && backup.pointer("/sections/presets").is_some() {
        sections.push("presets");
    }
    if selection.calibration && backup.pointer("/sections/calibration").is_some() {
        sections.push("calibration");
    }
    if selection.wifi && backup.pointer("/sections/settings/wifi").is_some() {
        sections.push("settings.wifi");
    }
    if selection.pd && backup.pointer("/sections/settings/pd").is_some() {
        sections.push("settings.pd");
    }
    sections
}

async fn disable_output_for_restore(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let control = request_api_value(
        client,
        default_devd,
        selector,
        reqwest::Method::POST,
        "/api/v1/control",
        Some(json!({"output_enabled": false})),
        false,
    )
    .await
    .map_err(|err| format!("safety_blocked: output disable failed: {err}"))?;
    if control.get("output_enabled").and_then(Value::as_bool) != Some(false) {
        return Err(format!("safety_blocked: output disable was not confirmed: {control}").into());
    }
    Ok(())
}

async fn restore_presets(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    backup: &Value,
    results: &mut Vec<Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let presets = backup
        .pointer("/sections/presets/presets")
        .and_then(Value::as_array)
        .ok_or("backup presets section must contain presets array")?;
    let current = request_api_value(
        client,
        default_devd,
        selector.clone(),
        reqwest::Method::GET,
        "/api/v1/presets",
        None,
        false,
    )
    .await?;
    let current_presets = current
        .get("presets")
        .and_then(Value::as_array)
        .ok_or("current presets response missing presets array")?;
    let mut changed_count = 0usize;
    for preset in presets {
        let preset_id = preset
            .get("preset_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        if current_presets.iter().any(|current| current == preset) {
            continue;
        }
        restore_preset_with_readback(client, default_devd, selector.clone(), preset, preset_id)
            .await
            .map_err(|err| format!("preset {preset_id} restore failed: {err}"))?;
        changed_count += 1;
        settle_after_restore_write().await;
    }
    results.push(
        json!({"section": "presets", "ok": true, "count": presets.len(), "changed": changed_count}),
    );
    Ok(())
}

async fn restore_preset_with_readback(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    preset: &Value,
    preset_id: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match request_api_value(
        client,
        default_devd,
        selector.clone(),
        reqwest::Method::POST,
        "/api/v1/presets",
        Some(preset.clone()),
        false,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(write_error) => {
            let readback = request_api_value(
                client,
                default_devd,
                selector,
                reqwest::Method::GET,
                "/api/v1/presets",
                None,
                false,
            )
            .await?;
            let restored = readback
                .get("presets")
                .and_then(Value::as_array)
                .and_then(|presets| {
                    presets.iter().find(|candidate| {
                        candidate.get("preset_id").and_then(Value::as_u64) == Some(preset_id)
                    })
                })
                == Some(preset);
            if restored { Ok(()) } else { Err(write_error) }
        }
    }
}

async fn restore_calibration(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    backup: &Value,
    results: &mut Vec<Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let calibration = backup
        .pointer("/sections/calibration")
        .ok_or("backup calibration section missing")?;
    const CURVES: [(&str, &str); 4] = [
        ("current_ch1", "current_ch1_points"),
        ("current_ch2", "current_ch2_points"),
        ("v_local", "v_local_points"),
        ("v_remote", "v_remote_points"),
    ];

    let all_empty = CURVES.iter().all(|(_, field)| {
        calibration
            .get(*field)
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    });
    let factory_default = calibration
        .pointer("/active/source")
        .and_then(Value::as_str)
        .is_some_and(|source| source == "factory-default" || source == "factory");
    if all_empty || factory_default {
        request_api_value(
            client,
            default_devd,
            selector,
            reqwest::Method::POST,
            "/api/v1/calibration/reset",
            Some(json!({"kind": "all"})),
            false,
        )
        .await?;
        results.push(json!({"section": "calibration", "ok": true, "action": "reset_all"}));
        return Ok(());
    }

    let mut actions = Vec::new();
    for (kind, field) in CURVES {
        let Some(points) = calibration.get(field) else {
            continue;
        };
        if points.as_array().is_none_or(Vec::is_empty) {
            request_api_value(
                client,
                default_devd,
                selector.clone(),
                reqwest::Method::POST,
                "/api/v1/calibration/reset",
                Some(json!({"kind": kind})),
                false,
            )
            .await?;
            settle_after_restore_write().await;
            actions.push(json!({"kind": kind, "action": "reset"}));
        } else {
            let body = calibration_curve_write_body(kind, points)?;
            request_api_value(
                client,
                default_devd,
                selector.clone(),
                reqwest::Method::POST,
                "/api/v1/calibration/commit",
                Some(body),
                false,
            )
            .await?;
            settle_after_restore_write().await;
            actions.push(json!({"kind": kind, "action": "commit"}));
        }
    }
    results.push(json!({"section": "calibration", "ok": true, "actions": actions}));
    Ok(())
}

async fn settle_after_restore_write() {
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
}

fn calibration_curve_write_body(
    kind: &str,
    points: &Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let array = points
        .as_array()
        .ok_or("calibration curve points must be an array")?;
    let mut compact_points = Vec::new();
    for point in array {
        if point.is_array() {
            compact_points.push(point.clone());
            continue;
        }
        let object = point
            .as_object()
            .ok_or("calibration point must be an object or compact array")?;
        let raw = object
            .get("raw_100uv")
            .or_else(|| object.get("raw"))
            .and_then(Value::as_i64)
            .ok_or("calibration point missing raw_100uv")?;
        if kind == "current_ch1" || kind == "current_ch2" {
            let dac = object
                .get("raw_dac_code")
                .or_else(|| object.get("dac_code"))
                .and_then(Value::as_u64)
                .ok_or("current calibration point missing raw_dac_code")?;
            let ma = object
                .get("meas_ma")
                .or_else(|| object.get("ma"))
                .and_then(Value::as_i64)
                .ok_or("current calibration point missing meas_ma")?;
            compact_points.push(json!([raw, dac, ma]));
        } else {
            let mv = object
                .get("meas_mv")
                .or_else(|| object.get("mv"))
                .and_then(Value::as_i64)
                .ok_or("voltage calibration point missing meas_mv")?;
            compact_points.push(json!([raw, mv]));
        }
    }
    Ok(json!({"kind": kind, "points": compact_points}))
}

async fn restore_wifi(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    backup: &Value,
    results: &mut Vec<Value>,
    allow_insecure_lan_wifi: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let wifi = backup
        .pointer("/sections/settings/wifi")
        .ok_or("backup settings.wifi section missing")?;
    let ssid = wifi
        .get("ssid")
        .and_then(Value::as_str)
        .ok_or("settings.wifi missing ssid")?;
    let psk = wifi
        .get("psk")
        .and_then(Value::as_str)
        .ok_or("settings.wifi missing psk")?;
    let source = wifi.get("source").and_then(Value::as_str).unwrap_or("user");
    if confirm_wifi_restore_readback(
        client,
        default_devd,
        selector.clone(),
        ssid,
        psk,
        source,
        allow_insecure_lan_wifi,
    )
    .await
    .is_ok()
    {
        results.push(json!({"section": "settings.wifi", "ok": true, "source": source, "action": "unchanged"}));
        return Ok(());
    }
    if source == "factory" {
        let res = request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::DELETE,
            "/api/v1/wifi",
            None,
            allow_insecure_lan_wifi,
        )
        .await;
        if let Err(err) = res {
            confirm_wifi_restore_readback(
                client,
                default_devd,
                selector,
                ssid,
                psk,
                "factory",
                allow_insecure_lan_wifi,
            )
            .await
            .map_err(|_| err)?;
        }
        results.push(json!({"section": "settings.wifi", "ok": true, "source": "factory", "action": "clear_user_override"}));
    } else {
        let res = request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::POST,
            "/api/v1/wifi",
            Some(json!({"ssid": ssid, "psk": psk, "wait": false})),
            allow_insecure_lan_wifi,
        )
        .await;
        if let Err(err) = res {
            confirm_wifi_restore_readback(
                client,
                default_devd,
                selector,
                ssid,
                psk,
                source,
                allow_insecure_lan_wifi,
            )
            .await
            .map_err(|_| err)?;
        }
        results.push(json!({"section": "settings.wifi", "ok": true, "source": wifi.get("source")}));
    }
    Ok(())
}

async fn confirm_wifi_restore_readback(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    ssid: &str,
    psk: &str,
    source: &str,
    allow_insecure_lan_wifi: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let readback = request_api_value(
        client,
        default_devd,
        selector,
        reqwest::Method::GET,
        "/api/v1/wifi/credentials",
        None,
        allow_insecure_lan_wifi,
    )
    .await?;
    let matches = readback.get("ssid").and_then(Value::as_str) == Some(ssid)
        && readback.get("psk").and_then(Value::as_str) == Some(psk)
        && readback.get("source").and_then(Value::as_str) == Some(source);
    if matches {
        Ok(())
    } else {
        Err(format!("WiFi readback did not match restored {source} credentials").into())
    }
}

async fn restore_pd(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    backup: &Value,
    results: &mut Vec<Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pd = backup
        .pointer("/sections/settings/pd")
        .ok_or("backup settings.pd section missing")?;
    let mut body = serde_json::Map::new();
    if let Some(saved) = pd.get("saved").and_then(Value::as_object) {
        if let Some(mode) = saved.get("mode").and_then(Value::as_str) {
            body.insert("mode".to_string(), json!(mode));
            match mode {
                "fixed" => {
                    if let Some(object_pos) = saved
                        .get("fixed_object_pos")
                        .or_else(|| saved.get("object_pos"))
                        .cloned()
                    {
                        body.insert("object_pos".to_string(), object_pos);
                    }
                    if let Some(target_mv) = saved.get("target_mv").cloned() {
                        body.insert("target_mv".to_string(), target_mv);
                    }
                }
                "pps" => {
                    if let Some(object_pos) = saved
                        .get("pps_object_pos")
                        .or_else(|| saved.get("object_pos"))
                        .cloned()
                    {
                        body.insert("object_pos".to_string(), object_pos);
                    }
                    if let Some(target_mv) = saved
                        .get("pps_target_mv")
                        .or_else(|| saved.get("target_mv"))
                        .cloned()
                    {
                        body.insert("target_mv".to_string(), target_mv);
                    }
                }
                _ => {}
            }
        }
        if let Some(i_req_ma) = saved.get("i_req_ma").cloned() {
            body.insert("i_req_ma".to_string(), i_req_ma);
        }
    }
    if let Some(allow) = pd.get("allow_extended_voltage").cloned() {
        body.insert("allow_extended_voltage".to_string(), allow);
    }
    request_api_value(
        client,
        default_devd,
        selector,
        reqwest::Method::POST,
        "/api/v1/pd",
        Some(Value::Object(body)),
        false,
    )
    .await?;
    results.push(json!({"section": "settings.pd", "ok": true}));
    Ok(())
}

async fn request_api_value(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    allow_insecure_lan_wifi: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    ensure_one_api_selector(
        selector.url.as_ref(),
        selector.device.as_ref(),
        selector.hardware.as_ref(),
    )?;
    let is_wifi_write = path == "/api/v1/wifi"
        && (method == reqwest::Method::POST || method == reqwest::Method::DELETE);
    if let Some(hardware_id) = selector.hardware {
        match resolve_saved_hardware(&hardware_id, default_devd)? {
            ResolvedHardware::Usb(resolved) => {
                let value = request_devd_usb_value(client, &resolved, method, path, body).await?;
                let _ = mark_hardware_transport_used(&resolved.hardware_id, SavedTransport::Usb);
                Ok(value)
            }
            ResolvedHardware::Http { url } => {
                if is_wifi_write && !allow_insecure_lan_wifi {
                    return Err("LAN WiFi writes require --allow-insecure-lan-wifi".into());
                }
                let value = request_http_value(client, &url, method, path, body).await?;
                let _ = mark_hardware_transport_used(&hardware_id, SavedTransport::Http);
                Ok(value)
            }
        }
    } else if let Some(url) = selector.url {
        if is_wifi_write && !allow_insecure_lan_wifi {
            return Err("LAN WiFi writes require --allow-insecure-lan-wifi".into());
        }
        request_http_value(client, &url, method, path, body).await
    } else if let Some(device) = selector.device {
        Err(format!(
            "temporary devd device id `{device}` cannot be used for operations; bind it first with `loadlynx hardware bind usb --candidate {device}` and then use --hardware <hardware-id>"
        )
        .into())
    } else {
        match resolve_saved_hardware("default", default_devd)? {
            ResolvedHardware::Usb(resolved) => {
                let value = request_devd_usb_value(client, &resolved, method, path, body).await?;
                let _ = mark_hardware_transport_used(&resolved.hardware_id, SavedTransport::Usb);
                Ok(value)
            }
            ResolvedHardware::Http { url } => {
                if is_wifi_write && !allow_insecure_lan_wifi {
                    return Err("LAN WiFi writes require --allow-insecure-lan-wifi".into());
                }
                let value = request_http_value(client, &url, method, path, body).await?;
                let _ = mark_default_transport_used(SavedTransport::Http);
                Ok(value)
            }
        }
    }
}

async fn request_http_value(
    client: &Client,
    base: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut request = client.request(method, api_url(base, path)?);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await?;
    let status = response.status();
    let value = response.json::<Value>().await?;
    if status.is_success() {
        Ok(value)
    } else {
        Err(format!("HTTP {status}: {value}").into())
    }
}

async fn request_devd_usb_value(
    client: &Client,
    resolved: &ResolvedUsbHardware,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let max_attempts = 5;
    for attempt in 1..=max_attempts {
        match request_devd_usb_value_once(
            client,
            resolved,
            method.clone(),
            path,
            body.clone(),
            attempt < max_attempts,
        )
        .await
        {
            Ok(value) => return Ok(value),
            Err(error) if attempt < max_attempts && is_retryable_devd_usb_error(&*error) => {
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
            }
            Err(error) => return Err(error),
        }
    }
    Err("devd USB request retry loop exhausted".into())
}

fn is_retryable_devd_usb_error(error: &(dyn std::error::Error + Send + Sync)) -> bool {
    let message = error.to_string();
    message.contains("retryable 503") || message.contains("503 Service Unavailable")
}

fn resolve_scanned_usb_device_for_saved_hardware(
    resolved: &ResolvedUsbHardware,
    scan: &Value,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let Some(saved_port_path) = resolved.port_path.as_deref() else {
        return Ok(resolved.device.clone());
    };
    let devices = scan
        .get("devices")
        .and_then(Value::as_array)
        .ok_or("devd scan response did not include devices")?;
    devices
        .iter()
        .find(|device| {
            device
                .get("digital_target")
                .and_then(|target| target.get("port_path"))
                .and_then(Value::as_str)
                == Some(saved_port_path)
        })
        .and_then(|device| device.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "saved USB hardware {} was not found at saved port path {} after devd scan",
                resolved.hardware_id, saved_port_path
            )
            .into()
        })
}

async fn request_devd_usb_value_once(
    client: &Client,
    resolved: &ResolvedUsbHardware,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    retryable_503: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let (lease, lease_device) = create_cli_lease_for_resolved_usb(client, resolved).await?;
    let heartbeat = spawn_cli_lease_heartbeat(client.clone(), resolved.devd.clone(), lease.clone());
    let separator = if path.contains('?') { '&' } else { '?' };
    let path = format!(
        "{path}{separator}device_id={}&lease_id={}",
        lease_device, lease.lease_id
    );
    let result: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
        match request_devd_value(&resolved.devd, method, &path, body).await {
            Ok(value) => Ok(value),
            Err(error) if retryable_503 && error.to_string().contains("HTTP 503") => {
                Err(format!("retryable 503 from devd USB request: {error}").into())
            }
            Err(error) => Err(error),
        }
    }
    .await;
    let _ = release_cli_lease(client, &resolved.devd, &lease.lease_id).await;
    heartbeat.abort();
    result
}

fn ensure_one_api_selector(
    url: Option<&String>,
    device: Option<&String>,
    hardware: Option<&String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let count = [url.is_some(), device.is_some(), hardware.is_some()]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    match count {
        0 => Ok(()),
        1 => Ok(()),
        _ => Err("command accepts only one of --hardware, --device, or --url".into()),
    }
}

fn resolve_output_enable(
    enable: bool,
    disable: bool,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match (enable, disable) {
        (true, false) => Ok(true),
        (false, true) => Ok(false),
        (true, true) => Err("command accepts only one of --enable or --disable".into()),
        (false, false) => Err("command requires --enable or --disable".into()),
    }
}

async fn create_cli_lease_with_expected(
    _client: &Client,
    devd: &str,
    device: &str,
    expected_identity_device_id: Option<&str>,
) -> Result<CliLease, Box<dyn std::error::Error + Send + Sync>> {
    let body = match expected_identity_device_id {
        Some(expected) => json!({"device_id": device, "expected_identity_device_id": expected}),
        None => json!({"device_id": device}),
    };
    Ok(serde_json::from_value(
        request_devd_value(
            devd,
            reqwest::Method::POST,
            "/api/v1/serial/lease",
            Some(body),
        )
        .await?,
    )?)
}

async fn create_cli_lease_for_resolved_usb(
    client: &Client,
    resolved: &ResolvedUsbHardware,
) -> Result<(CliLease, String), Box<dyn std::error::Error + Send + Sync>> {
    match create_cli_lease_with_expected(
        client,
        &resolved.devd,
        &resolved.device,
        Some(&resolved.expected_identity_device_id),
    )
    .await
    {
        Ok(lease) => Ok((lease, resolved.device.clone())),
        Err(error) if error.to_string().contains("device_not_found") => {
            let scan = request_devd_value(
                &resolved.devd,
                reqwest::Method::POST,
                "/api/v1/devices/scan",
                None,
            )
            .await?;
            let device = resolve_scanned_usb_device_for_saved_hardware(resolved, &scan)?;
            let lease = create_cli_lease_with_expected(
                client,
                &resolved.devd,
                &device,
                Some(&resolved.expected_identity_device_id),
            )
            .await?;
            Ok((lease, device))
        }
        Err(error) => Err(error),
    }
}

async fn create_cli_bind_probe_lease(
    _client: &Client,
    devd: &str,
    device: &str,
) -> Result<CliLease, Box<dyn std::error::Error + Send + Sync>> {
    Ok(serde_json::from_value(
        request_devd_value(
            devd,
            reqwest::Method::POST,
            "/api/v1/serial/lease",
            Some(json!({"device_id": device, "bind_probe": true})),
        )
        .await?,
    )?)
}

async fn release_cli_lease(
    _client: &Client,
    devd: &str,
    lease_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = request_devd_value(
        devd,
        reqwest::Method::DELETE,
        &format!("/api/v1/serial/lease/{lease_id}"),
        None,
    )
    .await;
    Ok(())
}

fn spawn_cli_lease_heartbeat(
    _client: Client,
    devd: String,
    lease: CliLease,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval_ms = (lease.heartbeat_interval_ms / 2).max(500);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            if request_devd_value(
                &devd,
                reqwest::Method::POST,
                &format!("/api/v1/serial/lease/{}", lease.lease_id),
                None,
            )
            .await
            .is_err()
            {
                break;
            }
        }
    })
}

async fn post_usb_operation_with_optional_lease(
    client: &Client,
    resolved: &ResolvedUsbHardware,
    path: &str,
    mut payload: Value,
    dry_run: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let lease = if dry_run {
        None
    } else {
        Some(create_cli_lease_for_resolved_usb(client, resolved).await?)
    };
    let heartbeat = lease.as_ref().map(|lease| {
        spawn_cli_lease_heartbeat(client.clone(), resolved.devd.clone(), lease.0.clone())
    });
    let operation_path = if let Some((_, lease_device)) = lease.as_ref() {
        path.replacen(
            &format!("/devices/{}", resolved.device),
            &format!("/devices/{lease_device}"),
            1,
        )
    } else {
        path.to_string()
    };
    if let Some((lease, _)) = lease.as_ref()
        && let Some(object) = payload.as_object_mut()
    {
        object.insert(
            "lease_id".to_string(),
            Value::String(lease.lease_id.clone()),
        );
    }

    let result: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
        request_devd_value(
            &resolved.devd,
            reqwest::Method::POST,
            &operation_path,
            Some(payload),
        )
        .await
    }
    .await;

    if let Some((lease, _)) = lease.as_ref() {
        let _ = release_cli_lease(client, &resolved.devd, &lease.lease_id).await;
    }
    if let Some(heartbeat) = heartbeat {
        heartbeat.abort();
    }
    result
}

async fn run_monitor(
    client: &Client,
    resolved: ResolvedUsbHardware,
    tail: usize,
    format: MonitorFormat,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let (lease, lease_device) = create_cli_lease_for_resolved_usb(client, &resolved).await?;
    let _heartbeat =
        spawn_cli_lease_heartbeat(client.clone(), resolved.devd.clone(), lease.clone());
    let mut seen = HashSet::new();
    loop {
        let session = request_devd_value(
            &resolved.devd,
            reqwest::Method::GET,
            &format!(
                "/api/v1/devices/{}/session?logs_limit={tail}&trace_limit={}&lease_id={}",
                lease_device,
                tail * 2,
                lease.lease_id
            ),
            None,
        )
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
                Some(
                    match request_devd_value(
                        devd,
                        reqwest::Method::POST,
                        "/api/v1/devices/scan",
                        None,
                    )
                    .await
                    {
                        Ok(value) => json!({"ok": true, "response": value}),
                        Err(error) => devd_error_payload(error),
                    },
                )
            } else {
                None
            };
            let devd_devices =
                match request_devd_value(devd, reqwest::Method::GET, "/api/v1/devices", None).await
                {
                    Ok(value) => json!({"ok": true, "response": value}),
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
        HardwareCommand::Bind {
            transport,
            candidate,
            url,
            name,
            set_default,
        } => {
            let mut registry = read_hardware_registry(&path)?;
            let now = current_unix_seconds();
            let saved = match transport {
                SavedTransport::Usb => {
                    let candidate = candidate.ok_or(
                        "USB bind requires --candidate <scan-candidate-id>; run `loadlynx hardware available --scan --json` first",
                    )?;
                    let scan = request_devd_value(
                        devd,
                        reqwest::Method::POST,
                        "/api/v1/devices/scan",
                        None,
                    )
                    .await?;
                    let device_record = scan
                        .get("devices")
                        .and_then(Value::as_array)
                        .and_then(|devices| {
                            devices.iter().find(|device| {
                                device.get("id").and_then(Value::as_str) == Some(candidate.as_str())
                            })
                        })
                        .ok_or_else(|| format!("USB candidate not found: {candidate}"))?;
                    let port_path = device_record
                        .get("digital_target")
                        .and_then(|target| target.get("port_path"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let identity = read_usb_identity_for_bind(client, devd, &candidate).await?;
                    let hardware_id = stable_hardware_id_from_identity(&identity)?;
                    upsert_hardware_transport(
                        &mut registry,
                        hardware_id,
                        name,
                        Some(identity),
                        SavedTransport::Usb,
                        Some(SavedUsbTransport {
                            device: candidate,
                            port_path,
                            devd: None,
                        }),
                        None,
                        now,
                    )
                }
                SavedTransport::Http => {
                    let url = url.ok_or("HTTP bind requires --url <base-url>")?;
                    let identity = request_http_value(
                        client,
                        &url,
                        reqwest::Method::GET,
                        "/api/v1/identity",
                        None,
                    )
                    .await?;
                    let hardware_id = stable_hardware_id_from_identity(&identity)?;
                    upsert_hardware_transport(
                        &mut registry,
                        hardware_id,
                        name,
                        Some(identity),
                        SavedTransport::Http,
                        None,
                        Some(SavedHttpTransport { url }),
                        now,
                    )
                }
            };
            if set_default || registry.default_hardware_id.is_none() {
                registry.default_hardware_id = Some(saved.id.clone());
            }
            write_hardware_registry(&path, &registry)?;
            Ok(
                json!({"path": path, "hardware": saved, "default_hardware_id": registry.default_hardware_id}),
            )
        }
        HardwareCommand::Default { command } => match command {
            HardwareDefaultCommand::Show => {
                let registry = read_hardware_registry(&path)?;
                let hardware = registry
                    .default_hardware_id
                    .as_ref()
                    .and_then(|id| registry.hardware.iter().find(|hardware| hardware.id == *id));
                Ok(
                    json!({"path": path, "default_hardware_id": registry.default_hardware_id, "hardware": hardware}),
                )
            }
            HardwareDefaultCommand::Set { id } => {
                let mut registry = read_hardware_registry(&path)?;
                if !registry.hardware.iter().any(|hardware| hardware.id == id) {
                    return Err(format!("saved hardware not found: {id}").into());
                }
                registry.default_hardware_id = Some(id.clone());
                write_hardware_registry(&path, &registry)?;
                Ok(json!({"path": path, "default_hardware_id": id}))
            }
            HardwareDefaultCommand::Clear => {
                let mut registry = read_hardware_registry(&path)?;
                registry.default_hardware_id = None;
                write_hardware_registry(&path, &registry)?;
                Ok(json!({"path": path, "default_hardware_id": Value::Null}))
            }
        },
        HardwareCommand::Use { id, transport } => {
            let mut registry = read_hardware_registry(&path)?;
            let hardware = registry
                .hardware
                .iter_mut()
                .find(|hardware| hardware.id == id)
                .ok_or_else(|| format!("saved hardware not found: {id}"))?;
            ensure_hardware_has_transport(hardware, transport)?;
            hardware.last_transport = Some(transport);
            hardware.last_seen_unix_seconds = Some(current_unix_seconds());
            let saved = hardware.clone();
            write_hardware_registry(&path, &registry)?;
            Ok(json!({"path": path, "hardware": saved}))
        }
        HardwareCommand::List => {
            let mut registry = read_hardware_registry(&path)?;
            sort_hardware(&mut registry.hardware);
            Ok(
                json!({"path": path, "default_hardware_id": registry.default_hardware_id, "hardware": registry.hardware}),
            )
        }
        HardwareCommand::Path => Ok(json!({"path": path})),
        HardwareCommand::Forget { id } => {
            let mut registry = read_hardware_registry(&path)?;
            let before = registry.hardware.len();
            registry.hardware.retain(|hardware| hardware.id != id);
            let removed = registry.hardware.len() != before;
            if registry.default_hardware_id.as_deref() == Some(id.as_str()) {
                registry.default_hardware_id = None;
            }
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
    let id = if id == "default" {
        registry.default_hardware_id.as_deref().ok_or(
            "default hardware is not set; run `loadlynx hardware default set <hardware-id>`",
        )?
    } else {
        id
    };
    let hardware = registry
        .hardware
        .iter()
        .find(|hardware| hardware.id == id)
        .ok_or_else(|| format!("saved hardware not found: {id}"))?;

    let transport = hardware
        .last_transport
        .ok_or_else(|| format!("saved hardware {id} has no selected transport; run `loadlynx hardware use {id} --transport <usb|http>`"))?;
    resolve_hardware_transport(hardware, transport, default_devd)
}

fn resolve_hardware_transport(
    hardware: &SavedHardware,
    transport: SavedTransport,
    default_devd: &str,
) -> Result<ResolvedHardware, Box<dyn std::error::Error + Send + Sync>> {
    match transport {
        SavedTransport::Usb => {
            let usb =
                hardware.transports.usb.as_ref().ok_or_else(|| {
                    format!("saved hardware {} has no USB transport", hardware.id)
                })?;
            Ok(ResolvedHardware::Usb(ResolvedUsbHardware {
                hardware_id: hardware.id.clone(),
                device: usb.device.clone(),
                devd: usb.devd.clone().unwrap_or_else(|| default_devd.to_string()),
                port_path: usb.port_path.clone(),
                expected_identity_device_id: hardware.id.clone(),
            }))
        }
        SavedTransport::Http => {
            let http =
                hardware.transports.http.as_ref().ok_or_else(|| {
                    format!("saved hardware {} has no HTTP transport", hardware.id)
                })?;
            Ok(ResolvedHardware::Http {
                url: http.url.clone(),
            })
        }
    }
}

fn resolve_usb_target(
    device: Option<String>,
    hardware: Option<String>,
    default_devd: &str,
) -> Result<ResolvedUsbHardware, Box<dyn std::error::Error + Send + Sync>> {
    if device.is_some() {
        return Err("temporary devd --device ids can only be used during `loadlynx hardware bind`; bind the hardware first and use --hardware <hardware-id>".into());
    }
    if let Some(hardware_id) = hardware {
        match resolve_saved_hardware(&hardware_id, default_devd)? {
            ResolvedHardware::Usb(resolved) => Ok(resolved),
            ResolvedHardware::Http { .. } => Err(format!(
                "saved hardware {hardware_id} uses HTTP; this command requires USB/devd hardware"
            )
            .into()),
        }
    } else {
        match resolve_saved_hardware("default", default_devd)? {
            ResolvedHardware::Usb(resolved) => Ok(resolved),
            ResolvedHardware::Http { .. } => {
                Err("default hardware uses HTTP; this command requires USB/devd hardware".into())
            }
        }
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
        0 => Ok(()),
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
        _ => Ok(()),
    }
}

fn mark_default_transport_used(
    transport: SavedTransport,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let registry = read_hardware_registry(&path)?;
    let Some(id) = registry.default_hardware_id.clone() else {
        return Ok(());
    };
    drop(registry);
    mark_hardware_transport_used(&id, transport)
}

fn mark_hardware_transport_used(
    id: &str,
    transport: SavedTransport,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let mut registry = read_hardware_registry(&path)?;
    let Some(hardware) = registry
        .hardware
        .iter_mut()
        .find(|hardware| hardware.id == id)
    else {
        return Ok(());
    };
    ensure_hardware_has_transport(hardware, transport)?;
    hardware.last_transport = Some(transport);
    hardware.last_seen_unix_seconds = Some(current_unix_seconds());
    write_hardware_registry(&path, &registry)
}

fn upsert_hardware_transport(
    registry: &mut HardwareRegistry,
    id: String,
    name: Option<String>,
    identity: Option<Value>,
    transport: SavedTransport,
    usb: Option<SavedUsbTransport>,
    http: Option<SavedHttpTransport>,
    now: u64,
) -> SavedHardware {
    if let Some(existing) = registry
        .hardware
        .iter_mut()
        .find(|existing| existing.id == id)
    {
        if name.is_some() {
            existing.name = name;
        }
        if identity.is_some() {
            existing.identity = identity;
        }
        match transport {
            SavedTransport::Usb => existing.transports.usb = usb,
            SavedTransport::Http => existing.transports.http = http,
        }
        existing.last_transport = Some(transport);
        existing.last_seen_unix_seconds = Some(now);
        let hardware = existing.clone();
        sort_hardware(&mut registry.hardware);
        return hardware;
    }
    let mut transports = SavedTransports::default();
    match transport {
        SavedTransport::Usb => transports.usb = usb,
        SavedTransport::Http => transports.http = http,
    }
    let hardware = SavedHardware {
        id,
        name,
        identity,
        last_transport: Some(transport),
        transports,
        last_seen_unix_seconds: Some(now),
    };
    registry.hardware.push(hardware.clone());
    sort_hardware(&mut registry.hardware);
    hardware
}

fn sort_hardware(hardware: &mut [SavedHardware]) {
    hardware.sort_by(|left, right| left.id.cmp(&right.id));
}

fn ensure_hardware_has_transport(
    hardware: &SavedHardware,
    transport: SavedTransport,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match transport {
        SavedTransport::Usb if hardware.transports.usb.is_none() => {
            Err(format!("saved hardware {} has no USB transport", hardware.id).into())
        }
        SavedTransport::Http if hardware.transports.http.is_none() => {
            Err(format!("saved hardware {} has no HTTP transport", hardware.id).into())
        }
        _ => Ok(()),
    }
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

    json!({
        "path": path,
        "devd": devd,
        "default_hardware_id": registry.default_hardware_id,
        "scan_requested": scan,
        "scan": scan_result,
        "usb": {
            "devices": devd_devices,
            "remembered": registry.hardware.iter().filter(|hardware| hardware.transports.usb.is_some()).cloned().collect::<Vec<_>>(),
        },
        "http_fallback": registry.hardware.iter().filter(|hardware| hardware.transports.http.is_some()).cloned().collect::<Vec<_>>(),
        "hardware": registry.hardware,
    })
}

fn devd_error_payload(error: impl std::fmt::Display) -> Value {
    json!({"ok": false, "error": error.to_string()})
}

async fn read_usb_identity_for_bind(
    client: &Client,
    devd: &str,
    device: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let lease = create_cli_bind_probe_lease(client, devd, device).await?;
    let heartbeat = spawn_cli_lease_heartbeat(client.clone(), devd.to_string(), lease.clone());
    let path = format!(
        "/api/v1/identity?device_id={device}&lease_id={}",
        lease.lease_id
    );
    let identity = request_devd_value(devd, reqwest::Method::GET, &path, None).await;
    let _ = release_cli_lease(client, devd, &lease.lease_id).await;
    heartbeat.abort();
    identity
}

fn stable_hardware_id_from_identity(
    identity: &Value,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let id = identity
        .get("device_id")
        .and_then(Value::as_str)
        .ok_or("identity did not include stable device_id")?;
    if id.starts_with("loadlynx-") || id.starts_with("mock-") {
        Ok(id.to_string())
    } else {
        Err(format!(
            "identity device_id `{id}` is not a stable LoadLynx hardware id; update firmware before binding or controlling this device"
        )
        .into())
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
    let value: Value = serde_json::from_str(&content)?;
    if value.get("default_hardware_id").is_some()
        || value
            .get("schema_version")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            >= 2
    {
        return Ok(serde_json::from_value(value)?);
    }
    migrate_legacy_hardware_registry(value)
}

fn migrate_legacy_hardware_registry(
    value: Value,
) -> Result<HardwareRegistry, Box<dyn std::error::Error + Send + Sync>> {
    #[derive(Deserialize)]
    struct LegacyHardwareRegistry {
        #[serde(default)]
        hardware: Vec<LegacySavedHardware>,
    }

    let legacy: LegacyHardwareRegistry = serde_json::from_value(value)?;
    let mut registry = HardwareRegistry::default();
    for item in legacy.hardware {
        let now = item
            .last_seen_unix_seconds
            .unwrap_or_else(current_unix_seconds);
        match item.transport {
            SavedTransport::Usb => {
                let Some(device) = item.device else {
                    continue;
                };
                upsert_hardware_transport(
                    &mut registry,
                    item.id,
                    item.name,
                    None,
                    SavedTransport::Usb,
                    Some(SavedUsbTransport {
                        device,
                        port_path: None,
                        devd: item.devd,
                    }),
                    None,
                    now,
                );
            }
            SavedTransport::Http => {
                let Some(url) = item.url else {
                    continue;
                };
                upsert_hardware_transport(
                    &mut registry,
                    item.id,
                    item.name,
                    None,
                    SavedTransport::Http,
                    None,
                    Some(SavedHttpTransport { url }),
                    now,
                );
            }
        }
    }
    Ok(registry)
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

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hardware_registry_schema_version() -> u8 {
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, extract::State, http::StatusCode, routing::post};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Default)]
    struct TestHttpState {
        lease_creates: Arc<AtomicUsize>,
        scans: Arc<AtomicUsize>,
        operation_payloads: Arc<Mutex<Vec<Value>>>,
        lease_payloads: Arc<Mutex<Vec<Value>>>,
    }

    async fn spawn_test_http(state: TestHttpState) -> String {
        async fn create_lease(
            State(state): State<TestHttpState>,
            axum::Json(payload): axum::Json<Value>,
        ) -> axum::Json<Value> {
            state.lease_creates.fetch_add(1, Ordering::SeqCst);
            state
                .lease_payloads
                .lock()
                .expect("lease payloads lock")
                .push(payload);
            axum::Json(json!({"lease_id": "lease-1", "heartbeat_interval_ms": 1000}))
        }

        async fn heartbeat_lease() -> axum::Json<Value> {
            axum::Json(json!({"ok": true}))
        }

        async fn release_lease() -> StatusCode {
            StatusCode::NO_CONTENT
        }

        async fn operation(
            State(state): State<TestHttpState>,
            axum::Json(payload): axum::Json<Value>,
        ) -> axum::Json<Value> {
            state
                .operation_payloads
                .lock()
                .expect("operation payloads lock")
                .push(payload.clone());
            axum::Json(json!({"ok": true, "payload": payload}))
        }

        async fn status() -> axum::Json<Value> {
            axum::Json(json!({"ok": true, "link_up": true}))
        }

        async fn scan(State(state): State<TestHttpState>) -> axum::Json<Value> {
            state.scans.fetch_add(1, Ordering::SeqCst);
            axum::Json(
                json!({"devices": [{"id": "digital-1", "digital_target": {"port_path": "mock://esp32s3"}}]}),
            )
        }

        let app = Router::new()
            .route("/api/v1/serial/lease", post(create_lease))
            .route(
                "/api/v1/serial/lease/{lease_id}",
                post(heartbeat_lease).delete(release_lease),
            )
            .route("/api/v1/status", axum::routing::get(status))
            .route("/api/v1/devices/scan", post(scan))
            .route("/api/v1/devices/{device}/flash", post(operation))
            .route("/api/v1/devices/{device}/reset", post(operation))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn spawn_scan_required_test_http(state: TestHttpState) -> String {
        async fn create_lease_after_scan(
            State(state): State<TestHttpState>,
            axum::Json(payload): axum::Json<Value>,
        ) -> (StatusCode, axum::Json<Value>) {
            state.lease_creates.fetch_add(1, Ordering::SeqCst);
            state
                .lease_payloads
                .lock()
                .expect("lease payloads lock")
                .push(payload);
            if state.scans.load(Ordering::SeqCst) == 0 {
                (
                    StatusCode::NOT_FOUND,
                    axum::Json(json!({"code": "device_not_found"})),
                )
            } else {
                (
                    StatusCode::OK,
                    axum::Json(json!({"lease_id": "lease-1", "heartbeat_interval_ms": 1000})),
                )
            }
        }

        async fn heartbeat_lease() -> axum::Json<Value> {
            axum::Json(json!({"ok": true}))
        }

        async fn release_lease() -> StatusCode {
            StatusCode::NO_CONTENT
        }

        async fn status() -> axum::Json<Value> {
            axum::Json(json!({"ok": true, "link_up": true}))
        }

        async fn scan(State(state): State<TestHttpState>) -> axum::Json<Value> {
            state.scans.fetch_add(1, Ordering::SeqCst);
            axum::Json(
                json!({"devices": [{"id": "digital-current", "digital_target": {"port_path": "mock://esp32s3"}}]}),
            )
        }

        let app = Router::new()
            .route("/api/v1/serial/lease", post(create_lease_after_scan))
            .route(
                "/api/v1/serial/lease/{lease_id}",
                post(heartbeat_lease).delete(release_lease),
            )
            .route("/api/v1/status", axum::routing::get(status))
            .route("/api/v1/devices/scan", post(scan))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    #[test]
    fn backup_include_defaults_to_all_and_expands_settings() {
        let selection = parse_backup_selection(&[]).expect("default selection");
        assert!(selection.presets);
        assert!(selection.calibration);
        assert!(selection.wifi);
        assert!(selection.pd);

        let selection =
            parse_backup_selection(&["presets,settings".to_string()]).expect("settings include");
        assert!(selection.presets);
        assert!(!selection.calibration);
        assert!(selection.wifi);
        assert!(selection.pd);
    }

    #[test]
    fn backup_unknown_sections_are_warned_and_supported_sections_are_restorable() {
        let backup = json!({
            "kind": "loadlynx.backup",
            "schema_version": 1,
            "sections": {
                "presets": {"presets": []},
                "future": {},
                "settings": {
                    "wifi": {"ssid": "BenchNet", "psk": "secret", "source": "user"},
                    "sound": {"volume": 2}
                }
            }
        });
        let warnings = backup_unknown_section_warnings(&backup);
        assert_eq!(warnings.len(), 2);
        let sections = restorable_backup_sections(&backup, BackupSelection::all());
        assert_eq!(sections, vec!["presets", "settings.wifi"]);
    }

    #[test]
    fn calibration_backup_points_are_converted_to_compact_commit_body() {
        let body = calibration_curve_write_body(
            "current_ch1",
            &json!([
                {"raw_100uv": -12, "raw_dac_code": 0, "meas_ma": 0},
                {"raw_100uv": 25000, "raw_dac_code": 4095, "meas_ma": 5000}
            ]),
        )
        .expect("compact body");
        assert_eq!(
            body,
            json!({"kind": "current_ch1", "points": [[-12, 0, 0], [25000, 4095, 5000]]})
        );
    }

    #[tokio::test]
    async fn backup_import_dry_run_does_not_touch_device() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(
            &mut file,
            json!({
                "kind": "loadlynx.backup",
                "schema_version": 1,
                "sections": {
                    "settings": {
                        "wifi": {"ssid": "BenchNet", "psk": "secret", "source": "user"}
                    }
                }
            })
            .to_string()
            .as_bytes(),
        )
        .expect("write backup");

        let payload = handle_backup_import(
            &Client::new(),
            "http://127.0.0.1:9",
            ApiSelector {
                url: Some("http://127.0.0.1:9".to_string()),
                device: None,
                hardware: None,
            },
            file.path(),
            &[],
            true,
            false,
        )
        .await
        .expect("dry-run import");

        assert_eq!(payload["dry_run"], true);
        assert_eq!(payload["would_restore"], json!(["settings.wifi"]));
    }

    #[test]
    fn backup_dry_run_human_output_shows_preview() {
        let output = render_human_payload(&json!({
            "ok": true,
            "dry_run": true,
            "would_restore": ["presets", "settings.wifi"],
            "warnings": [{"message": "Unknown section ignored: settings.sound"}]
        }))
        .expect("human render");

        assert!(output.contains("Backup dry-run"));
        assert!(output.contains("presets, settings.wifi"));
        assert!(output.contains("Unknown section ignored: settings.sound"));
    }

    #[tokio::test]
    async fn backup_wifi_credentials_allows_lan_http_selector() {
        let err = request_api_value(
            &Client::new(),
            "http://127.0.0.1:9",
            ApiSelector {
                url: Some("http://127.0.0.1:9".to_string()),
                device: None,
                hardware: None,
            },
            reqwest::Method::GET,
            "/api/v1/wifi/credentials",
            None,
            false,
        )
        .await
        .expect_err("test server should be unreachable after LAN credentials are allowed");

        assert!(!err.to_string().contains("requires the local USB/devd path"));
    }

    #[cfg(unix)]
    #[test]
    fn backup_export_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("backup.json");
        write_backup_file(
            &path,
            &json!({
                "kind": "loadlynx.backup",
                "schema_version": 1,
                "sections": {
                    "settings": {
                        "wifi": {"ssid": "BenchNet", "psk": "secret", "source": "user"}
                    }
                }
            }),
        )
        .expect("write backup");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn backup_import_parses_lan_wifi_write_opt_in() {
        let cli = Cli::try_parse_from([
            "loadlynx",
            "backup",
            "import",
            "--url",
            "http://loadlynx.local",
            "--file",
            "-",
            "--allow-insecure-lan-wifi",
        ])
        .expect("backup import parse");
        match cli.command {
            Command::Backup {
                command:
                    BackupCommand::Import {
                        allow_insecure_lan_wifi,
                        ..
                    },
            } => assert!(allow_insecure_lan_wifi),
            _ => panic!("expected backup import command"),
        }
    }

    #[test]
    fn backup_import_preflight_rejects_lan_wifi_before_device_writes() {
        let backup = json!({
            "kind": "loadlynx.backup",
            "schema_version": 1,
            "sections": {
                "settings": {
                    "wifi": {"ssid": "BenchNet", "psk": "secret", "source": "user"}
                }
            }
        });
        let selector = ApiSelector {
            url: Some("http://loadlynx.local".to_string()),
            device: None,
            hardware: None,
        };
        let selection = parse_backup_selection(&[]).expect("default selection");

        let err =
            preflight_backup_restore("http://127.0.0.1:9", &selector, &backup, selection, false)
                .expect_err("LAN WiFi restore should require an explicit opt-in");

        assert!(
            err.to_string()
                .contains("LAN WiFi writes require --allow-insecure-lan-wifi")
        );
    }

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
    fn initial_devd_endpoints_skip_local_commands() {
        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "usb-port",
            "set",
            "digital",
            "/dev/cu.usbmodem212101",
        ])
        .expect("usb-port parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());

        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "hardware",
            "list",
        ])
        .expect("hardware list parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());

        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "hardware",
            "available",
        ])
        .expect("hardware available parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());
    }

    #[test]
    fn initial_devd_endpoints_include_usb_commands() {
        let cli = Cli::try_parse_from(["loadlynx", "--ipc", "/tmp/loadlynx.sock", "devices"])
            .expect("devices parse");
        assert_eq!(
            initial_devd_endpoints(&cli.command, &cli.ipc),
            vec!["/tmp/loadlynx.sock"]
        );

        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "status",
            "--device",
            "digital-1",
        ])
        .expect("status device parse");
        assert_eq!(
            initial_devd_endpoints(&cli.command, &cli.ipc),
            vec!["/tmp/loadlynx.sock"]
        );

        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "hardware",
            "available",
            "--scan",
        ])
        .expect("hardware available scan parse");
        assert_eq!(
            initial_devd_endpoints(&cli.command, &cli.ipc),
            vec!["/tmp/loadlynx.sock"]
        );
    }

    #[test]
    fn initial_devd_endpoints_skip_http_url_commands() {
        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "status",
            "--url",
            "http://loadlynx.local",
        ])
        .expect("status url parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());
    }

    #[tokio::test]
    async fn request_devd_value_accepts_legacy_http_endpoint() {
        let state = TestHttpState::default();
        let devd = spawn_test_http(state).await;

        let value = request_devd_value(
            &devd,
            reqwest::Method::POST,
            "/api/v1/devices/digital-1/reset",
            Some(json!({"dry_run": true})),
        )
        .await
        .unwrap();

        assert_eq!(value.get("ok").and_then(Value::as_bool), Some(true));
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
    fn analog_real_flash_does_not_require_digital_confirmation() {
        assert_eq!(
            resolve_flash_confirmation_text(&BoardTarget::Analog, false, None).unwrap(),
            None
        );
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

    #[tokio::test]
    async fn dry_run_usb_firmware_operation_does_not_create_cli_lease() {
        let state = TestHttpState::default();
        let devd = spawn_test_http(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "mock-loadlynx-devd".to_string(),
            device: "digital-1".to_string(),
            devd,
            port_path: None,
            expected_identity_device_id: "mock-loadlynx-devd".to_string(),
        };

        post_usb_operation_with_optional_lease(
            &Client::new(),
            &resolved,
            "/api/v1/devices/digital-1/flash",
            json!({"target": TargetKind::DigitalEsp32s3, "dry_run": true}),
            true,
        )
        .await
        .unwrap();

        assert_eq!(state.lease_creates.load(Ordering::SeqCst), 0);
        let payloads = state
            .operation_payloads
            .lock()
            .expect("operation payloads lock");
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0].get("dry_run").and_then(Value::as_bool),
            Some(true)
        );
        assert!(payloads[0].get("lease_id").is_none());
    }

    #[tokio::test]
    async fn real_usb_firmware_operation_creates_cli_lease() {
        let state = TestHttpState::default();
        let devd = spawn_test_http(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "mock-loadlynx-devd".to_string(),
            device: "digital-1".to_string(),
            devd,
            port_path: None,
            expected_identity_device_id: "mock-loadlynx-devd".to_string(),
        };

        post_usb_operation_with_optional_lease(
            &Client::new(),
            &resolved,
            "/api/v1/devices/digital-1/reset",
            json!({"target": TargetKind::DigitalEsp32s3, "dry_run": false}),
            false,
        )
        .await
        .unwrap();

        assert_eq!(state.lease_creates.load(Ordering::SeqCst), 1);
        let payloads = state
            .operation_payloads
            .lock()
            .expect("operation payloads lock");
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0].get("dry_run").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payloads[0].get("lease_id").and_then(Value::as_str),
            Some("lease-1")
        );
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
    fn hardware_registry_round_trips_v2_entities() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devices.json");
        let mut registry = HardwareRegistry::default();
        registry.default_hardware_id = Some("loadlynx-bench".to_string());
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-bench".to_string(),
            None,
            None,
            SavedTransport::Http,
            None,
            Some(SavedHttpTransport {
                url: "http://loadlynx.local".to_string(),
            }),
            10,
        );
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-bench".to_string(),
            Some("Bench".to_string()),
            None,
            SavedTransport::Usb,
            Some(SavedUsbTransport {
                device: "digital-1".to_string(),
                port_path: Some("/dev/cu.usbmodem1".to_string()),
                devd: None,
            }),
            None,
            20,
        );
        write_hardware_registry(&path, &registry).unwrap();

        let reloaded = read_hardware_registry(&path).unwrap();
        assert_eq!(
            reloaded.default_hardware_id.as_deref(),
            Some("loadlynx-bench")
        );
        assert_eq!(reloaded.hardware.len(), 1);
        assert_eq!(reloaded.hardware[0].id, "loadlynx-bench");
        assert_eq!(
            reloaded.hardware[0].last_transport,
            Some(SavedTransport::Usb)
        );
        assert!(reloaded.hardware[0].transports.usb.is_some());
        assert!(reloaded.hardware[0].transports.http.is_some());
    }

    #[test]
    fn legacy_hardware_registry_migrates_to_v2_entities() {
        let legacy = json!({
            "schema_version": 1,
            "hardware": [
                {
                    "id": "loadlynx-bench",
                    "name": "Bench",
                    "transport": "usb",
                    "device": "digital-1",
                    "devd": "http://127.0.0.1:30180",
                    "last_seen_unix_seconds": 10
                },
                {
                    "id": "loadlynx-bench",
                    "transport": "http",
                    "url": "http://loadlynx-bench.local",
                    "last_seen_unix_seconds": 20
                }
            ]
        });

        let migrated = migrate_legacy_hardware_registry(legacy).unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.hardware.len(), 1);
        assert_eq!(migrated.hardware[0].id, "loadlynx-bench");
        assert_eq!(
            migrated.hardware[0].last_transport,
            Some(SavedTransport::Http)
        );
        assert!(migrated.hardware[0].transports.usb.is_some());
        assert!(migrated.hardware[0].transports.http.is_some());
    }

    #[test]
    fn hardware_commands_parse_saved_device_workflows() {
        let cli = Cli::try_parse_from([
            "loadlynx",
            "hardware",
            "bind",
            "usb",
            "--candidate",
            "digital-1",
            "--set-default",
        ])
        .unwrap();
        match cli.command {
            Command::Hardware {
                command:
                    HardwareCommand::Bind {
                        transport,
                        candidate,
                        set_default,
                        ..
                    },
            } => {
                assert_eq!(transport, SavedTransport::Usb);
                assert_eq!(candidate.as_deref(), Some("digital-1"));
                assert!(set_default);
            }
            _ => panic!("expected hardware bind command"),
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

        let cli =
            Cli::try_parse_from(["loadlynx", "hardware", "default", "set", "loadlynx-abc123"])
                .unwrap();
        match cli.command {
            Command::Hardware {
                command:
                    HardwareCommand::Default {
                        command: HardwareDefaultCommand::Set { id },
                    },
            } => assert_eq!(id, "loadlynx-abc123"),
            _ => panic!("expected hardware default set command"),
        }

        let cli = Cli::try_parse_from([
            "loadlynx",
            "output",
            "set",
            "--hardware",
            "usb-digital-1",
            "--disable",
        ])
        .unwrap();
        match cli.command {
            Command::Output {
                command:
                    OutputCommand::Set {
                        hardware,
                        enable,
                        disable,
                        ..
                    },
            } => {
                assert_eq!(hardware.as_deref(), Some("usb-digital-1"));
                assert!(!enable);
                assert!(disable);
            }
            _ => panic!("expected output set command"),
        }

        assert!(
            Cli::try_parse_from([
                "loadlynx",
                "output",
                "set",
                "--hardware",
                "usb-digital-1",
                "--target-i-ma=-1",
                "--enable",
            ])
            .is_err()
        );

        let cli = Cli::try_parse_from([
            "loadlynx",
            "control",
            "set",
            "--hardware",
            "usb-digital-1",
            "--enable",
        ])
        .unwrap();
        match cli.command {
            Command::Control {
                command:
                    ControlCommand::Set {
                        hardware,
                        enable,
                        disable,
                        ..
                    },
            } => {
                assert_eq!(hardware.as_deref(), Some("usb-digital-1"));
                assert!(enable);
                assert!(!disable);
            }
            _ => panic!("expected control set command"),
        }
    }

    #[test]
    fn output_set_requires_exactly_one_enable_state() {
        assert_eq!(resolve_output_enable(true, false).unwrap(), true);
        assert_eq!(resolve_output_enable(false, true).unwrap(), false);
        assert!(resolve_output_enable(true, true).is_err());
        assert!(resolve_output_enable(false, false).is_err());
    }

    #[test]
    fn cli_errors_are_classified_for_json_automation() {
        assert_eq!(
            classify_cli_error_code("default hardware is not set; run bind"),
            "default_hardware_not_set"
        );
        assert_eq!(
            classify_cli_error_code(
                "identity device_id `digital-esp32s3` is not a stable LoadLynx hardware id"
            ),
            "unstable_hardware_identity"
        );
    }

    #[tokio::test]
    async fn bind_probe_lease_marks_explicit_binding_probe() {
        let state = TestHttpState::default();
        let devd = spawn_test_http(state.clone()).await;

        create_cli_bind_probe_lease(&Client::new(), &devd, "digital-1")
            .await
            .unwrap();

        let payloads = state.lease_payloads.lock().expect("lease payloads lock");
        assert_eq!(
            payloads[0].get("bind_probe").and_then(Value::as_bool),
            Some(true)
        );
        assert!(payloads[0].get("expected_identity_device_id").is_none());
    }

    #[tokio::test]
    async fn saved_usb_request_scans_fresh_devd_before_retrying_lease() {
        let state = TestHttpState::default();
        let devd = spawn_scan_required_test_http(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "loadlynx-bench".to_string(),
            device: "digital-stale".to_string(),
            devd,
            port_path: Some("mock://esp32s3".to_string()),
            expected_identity_device_id: "loadlynx-bench".to_string(),
        };

        let value = request_devd_usb_value(
            &Client::new(),
            &resolved,
            reqwest::Method::GET,
            "/api/v1/status",
            None,
        )
        .await
        .unwrap();

        assert_eq!(value.get("link_up").and_then(Value::as_bool), Some(true));
        assert_eq!(state.scans.load(Ordering::SeqCst), 1);
        assert_eq!(state.lease_creates.load(Ordering::SeqCst), 2);
        let payloads = state.lease_payloads.lock().expect("lease payloads lock");
        assert_eq!(
            payloads[1].get("device_id").and_then(Value::as_str),
            Some("digital-current")
        );
        assert_eq!(
            payloads[1]
                .get("expected_identity_device_id")
                .and_then(Value::as_str),
            Some("loadlynx-bench")
        );
    }

    #[test]
    fn hardware_use_updates_last_transport() {
        let mut hardware = vec![
            SavedHardware {
                id: "loadlynx-old".to_string(),
                name: None,
                identity: None,
                last_transport: Some(SavedTransport::Usb),
                transports: SavedTransports {
                    usb: Some(SavedUsbTransport {
                        device: "old".to_string(),
                        port_path: None,
                        devd: None,
                    }),
                    http: None,
                },
                last_seen_unix_seconds: Some(10),
            },
            SavedHardware {
                id: "loadlynx-new".to_string(),
                name: None,
                identity: None,
                last_transport: Some(SavedTransport::Http),
                transports: SavedTransports {
                    usb: None,
                    http: Some(SavedHttpTransport {
                        url: "http://new.local".to_string(),
                    }),
                },
                last_seen_unix_seconds: Some(30),
            },
        ];

        sort_hardware(&mut hardware);

        assert_eq!(
            hardware
                .iter()
                .map(|hardware| hardware.id.as_str())
                .collect::<Vec<_>>(),
            vec!["loadlynx-new", "loadlynx-old"]
        );
    }

    #[test]
    fn available_hardware_payload_keeps_usb_and_http_fallback_separate() {
        let mut registry = HardwareRegistry::default();
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-http".to_string(),
            None,
            None,
            SavedTransport::Http,
            None,
            Some(SavedHttpTransport {
                url: "http://loadlynx.local".to_string(),
            }),
            10,
        );
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-usb".to_string(),
            None,
            None,
            SavedTransport::Usb,
            Some(SavedUsbTransport {
                device: "digital-1".to_string(),
                port_path: None,
                devd: None,
            }),
            None,
            20,
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
            Some("loadlynx-usb")
        );
        assert_eq!(
            payload
                .pointer("/http_fallback/0/id")
                .and_then(Value::as_str),
            Some("loadlynx-http")
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
        assert!(usb_err.to_string().contains("bind the hardware first"));

        let output_err = ensure_one_output_selector(
            Some(&"http://loadlynx.local".to_string()),
            Some(&"bench".to_string()),
        )
        .unwrap_err();
        assert!(output_err.to_string().contains("only one"));
    }
}
