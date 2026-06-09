use chrono::Utc;
use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use loadlynx_devd::{
    FLASH_CONFIRMATION_TEXT, IpcRequest, TargetKind, default_ipc_endpoint, ipc_request,
    list_digital_usb_port_candidates, write_default_digital_usb_port,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::HashSet,
    env, fs, io,
    io::{IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::process::Command as TokioCommand;

#[path = "loadlynx/backup.rs"]
mod backup;
#[path = "loadlynx/hardware.rs"]
mod hardware;
#[path = "loadlynx/mode_first.rs"]
mod mode_first;
#[path = "loadlynx/render.rs"]
mod render;
#[path = "loadlynx/transport.rs"]
mod transport;

#[cfg(test)]
use backup::{
    BackupSelection, backup_unknown_section_warnings, calibration_curve_write_body,
    parse_backup_selection, preflight_backup_restore, restorable_backup_sections,
    write_backup_file,
};
use backup::{handle_backup_export, handle_backup_import};
#[cfg(test)]
use hardware::{
    HardwareRegistry, SavedHardware, SavedHttpTransport, SavedTransports, SavedUsbTransport,
    available_hardware_payload, hardware_registry_path_from_values,
    migrate_legacy_hardware_registry, read_hardware_registry, resolve_hardware_transport,
    sort_hardware, upsert_hardware_transport, write_hardware_registry,
};
use hardware::{
    ResolvedHardware, ResolvedUsbHardware, SavedTransport, handle_device_command,
    has_saved_device_for_transport, is_stable_hardware_id, mark_hardware_transport_used,
    resolve_saved_hardware_selection, resolve_saved_hardware_selection_with_transport,
    resolve_usb_target,
};
use mode_first::{ModeFirstCommand, handle_mode_first_command};
#[cfg(test)]
use mode_first::{handle_mode_first_command_for_selector, validate_mode_first_targets};
#[cfg(test)]
use render::{classify_cli_error_code, render_human_payload};
use render::{print_cli_error, print_cli_payload};
#[cfg(test)]
use transport::validate_cli_lease_identity;
use transport::{
    ApiSelector, create_cli_bind_probe_lease, ensure_one_api_selector, ensure_one_status_selector,
    freeze_api_selector, post_usb_operation_with_optional_lease, release_cli_lease,
    request_api_value, request_devd_usb_value, request_http_value, resolve_output_enable,
    resolve_scanned_usb_device_for_saved_hardware, run_monitor, saved_usb_device_needs_relookup,
    spawn_cli_lease_heartbeat,
};

#[derive(Debug, Parser)]
#[command(name = "loadlynx")]
#[command(about = "LoadLynx LAN/USB/devd control CLI")]
#[command(disable_version_flag = true)]
#[command(version)]
struct Cli {
    #[arg(long, global = true, default_value_t = default_ipc_endpoint(), hide = true)]
    ipc: String,
    #[arg(long, global = true, hide = true)]
    no_auto_start: bool,
    #[arg(short = 'v', long = "version", action = ArgAction::Version)]
    version: Option<bool>,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Completion {
        shell: Shell,
    },
    Devices,
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
    Status {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
    Flash {
        target: BoardTarget,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long = "manifest-path", hide = true)]
        manifest_path: Option<String>,
        #[arg(long = "no-dry-run", default_value_t = true, action = ArgAction::SetFalse)]
        dry_run: bool,
        #[arg(long = "confirm", alias = "confirm-phrase")]
        confirm: Option<String>,
        #[arg(long, hide = true)]
        expected_identity_device_id: Option<String>,
        #[arg(long)]
        acknowledge_non_project_firmware: bool,
    },
    Cc {
        target_i_ma: u32,
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        preset_id: Option<u8>,
        #[arg(long)]
        min_v_mv: Option<u32>,
        #[arg(long)]
        max_i_ma_total: Option<u32>,
        #[arg(long)]
        max_p_mw: Option<u32>,
        #[arg(long)]
        disable: bool,
    },
    Cv {
        target_v_mv: u32,
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        preset_id: Option<u8>,
        #[arg(long)]
        min_v_mv: Option<u32>,
        #[arg(long)]
        max_i_ma_total: Option<u32>,
        #[arg(long)]
        max_p_mw: Option<u32>,
        #[arg(long)]
        disable: bool,
    },
    Cp {
        target_p_mw: u32,
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        preset_id: Option<u8>,
        #[arg(long)]
        min_v_mv: Option<u32>,
        #[arg(long)]
        max_i_ma_total: Option<u32>,
        #[arg(long)]
        max_p_mw: Option<u32>,
        #[arg(long)]
        disable: bool,
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
    #[command(hide = true)]
    Discover {
        #[arg(long)]
        mdns: bool,
        #[arg(long)]
        lan_scan: bool,
    },
    #[command(hide = true)]
    Reset {
        target: BoardTarget,
        #[arg(long)]
        device: Option<String>,
        #[arg(long = "no-dry-run", default_value_t = true, action = ArgAction::SetFalse)]
        dry_run: bool,
    },
    #[command(hide = true)]
    Monitor {
        target: BoardTarget,
        #[arg(long)]
        device: Option<String>,
        #[arg(long, default_value_t = 200)]
        tail: usize,
        #[arg(long, value_enum, default_value_t = MonitorFormat::Human)]
        format: MonitorFormat,
    },
    #[command(hide = true)]
    Calibration {
        #[command(subcommand)]
        command: CalibrationCommand,
    },
    #[command(hide = true)]
    SoftReset {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long, default_value = "manual")]
        reason: String,
    },
    #[command(hide = true)]
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
    #[command(hide = true)]
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    #[command(hide = true)]
    UsbPort {
        #[command(subcommand)]
        command: UsbPortCommand,
    },
}

#[derive(Debug, Subcommand)]
enum DeviceCommand {
    List,
    Add {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    Use {
        id: Option<String>,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        clear: bool,
    },
    Remove {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum PdCommand {
    Set {
        #[arg(long)]
        device: Option<String>,
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
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
    Set {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
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
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        allow_insecure_lan_wifi: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ControlCommand {
    Get {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
    Set {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        disable: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PresetCommand {
    List {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
    Set {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        file: PathBuf,
    },
    Apply {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        preset_id: u8,
    },
}

#[derive(Debug, Subcommand)]
enum CalibrationCommand {
    Profile {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
    Mode {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        kind: String,
    },
    Apply {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        file: PathBuf,
    },
    Commit {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        file: PathBuf,
    },
    Reset {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        kind: String,
    },
}

#[derive(Debug, Subcommand)]
enum DiagnosticsCommand {
    Export {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum BackupCommand {
    Export {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(long = "include", value_delimiter = ',')]
        include: Vec<String>,
    },
    Import {
        #[arg(long, hide = true)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
struct CliLease {
    lease_id: String,
    identity_device_id: Option<String>,
    heartbeat_interval_ms: u64,
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
    let manifest = PathBuf::from("tools/loadlynx-devd/Cargo.toml");
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
        return Err(
            "devd endpoint must be a native IPC endpoint, not HTTP; use --url or a saved HTTP device transport for LAN devices"
                .into(),
        );
    }

    let request = ipc_request_for_devd_call(method, path, body)?;
    let response = ipc_request(endpoint, request).await?;
    if response.ok {
        Ok(response.result.unwrap_or(Value::Null))
    } else {
        let error = response
            .error
            .map(|error| serde_json::to_string(&error).unwrap_or_else(|_| "<invalid error>".into()))
            .unwrap_or_else(|| "unknown devd IPC error".to_string());
        Err(format!("devd IPC operation failed: {error}").into())
    }
}

fn ipc_request_for_devd_call(
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<IpcRequest, Box<dyn std::error::Error + Send + Sync>> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let mut params = parse_query_params(query)?;
    let segments = route
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let op = match (method.as_str(), segments.as_slice()) {
        ("GET", ["health"]) | ("GET", ["api", "v1", "ping"]) => "health",
        ("GET", ["api", "v1", "devices"]) => "devices.list",
        ("POST", ["api", "v1", "devices", "scan"]) => "devices.scan",
        ("POST", ["api", "v1", "devices", id, "artifact"]) => {
            params.insert("device_id".to_string(), json!(id));
            merge_body_object(&mut params, body)?;
            return Ok(IpcRequest {
                op: "devices.artifact.select".to_string(),
                params: Value::Object(params),
            });
        }
        ("POST", ["api", "v1", "devices", id, "flash"]) => {
            params.insert("device_id".to_string(), json!(id));
            merge_body_object(&mut params, body)?;
            return Ok(IpcRequest {
                op: "devices.flash".to_string(),
                params: Value::Object(params),
            });
        }
        ("POST", ["api", "v1", "devices", id, "reset"]) => {
            params.insert("device_id".to_string(), json!(id));
            merge_body_object(&mut params, body)?;
            return Ok(IpcRequest {
                op: "devices.reset".to_string(),
                params: Value::Object(params),
            });
        }
        ("GET", ["api", "v1", "devices", id, "session"]) => {
            params.insert("device_id".to_string(), json!(id));
            "devices.session"
        }
        ("POST", ["api", "v1", "serial", "lease"]) => {
            merge_body_object(&mut params, body)?;
            return Ok(IpcRequest {
                op: "serial.lease.create".to_string(),
                params: Value::Object(params),
            });
        }
        ("POST", ["api", "v1", "serial", "lease", lease_id]) => {
            params.insert("lease_id".to_string(), json!(lease_id));
            "serial.lease.heartbeat"
        }
        ("DELETE", ["api", "v1", "serial", "lease", lease_id]) => {
            params.insert("lease_id".to_string(), json!(lease_id));
            "serial.lease.release"
        }
        ("GET", ["api", "v1", "identity"]) => "compat.identity",
        ("GET", ["api", "v1", "status"]) => "compat.status",
        ("GET", ["api", "v1", "network"]) => "compat.network",
        ("GET", ["api", "v1", "serial", "session"]) => "compat.session",
        ("GET", ["api", "v1", "pd"]) => "compat.pd.get",
        ("POST", ["api", "v1", "pd"]) | ("PUT", ["api", "v1", "pd"]) => {
            set_body(&mut params, body.as_ref());
            "compat.pd.post"
        }
        ("POST", ["api", "v1", "cc"]) | ("PUT", ["api", "v1", "cc"]) => {
            set_body(&mut params, body.as_ref());
            "compat.cc"
        }
        ("GET", ["api", "v1", "wifi"]) => "compat.wifi.get",
        ("POST", ["api", "v1", "wifi"]) | ("PUT", ["api", "v1", "wifi"]) => {
            set_body(&mut params, body.as_ref());
            "compat.wifi.post"
        }
        ("DELETE", ["api", "v1", "wifi"]) => "compat.wifi.delete",
        ("GET", ["api", "v1", "wifi", "credentials"]) => "compat.wifi.credentials",
        ("GET", ["api", "v1", "control"]) => "compat.control.get",
        ("POST", ["api", "v1", "control"]) | ("PUT", ["api", "v1", "control"]) => {
            set_body(&mut params, body.as_ref());
            "compat.control.post"
        }
        ("GET", ["api", "v1", "presets"]) => "compat.presets.get",
        ("POST", ["api", "v1", "presets"]) | ("PUT", ["api", "v1", "presets"]) => {
            set_body(&mut params, body.as_ref());
            "compat.presets.post"
        }
        ("POST", ["api", "v1", "presets", "apply"]) => {
            set_body(&mut params, body.as_ref());
            "compat.presets.apply"
        }
        ("GET", ["api", "v1", "calibration", "profile"]) => "compat.calibration.profile",
        ("POST", ["api", "v1", "calibration", "apply"]) => {
            set_body(&mut params, body.as_ref());
            "compat.calibration.apply"
        }
        ("POST", ["api", "v1", "calibration", "commit"]) => {
            set_body(&mut params, body.as_ref());
            "compat.calibration.commit"
        }
        ("POST", ["api", "v1", "calibration", "reset"]) => {
            set_body(&mut params, body.as_ref());
            "compat.calibration.reset"
        }
        ("POST", ["api", "v1", "calibration", "mode"]) => {
            set_body(&mut params, body.as_ref());
            "compat.calibration.mode"
        }
        ("POST", ["api", "v1", "soft-reset"]) => {
            set_body(&mut params, body.as_ref());
            "compat.soft_reset"
        }
        ("GET", ["api", "v1", "diagnostics"]) | ("GET", ["api", "v1", "diagnostics", "export"]) => {
            "compat.diagnostics.export"
        }
        _ => {
            return Err(format!("unsupported devd IPC route: {} {}", method.as_str(), path).into());
        }
    };

    if body.is_some() {
        set_body(&mut params, body.as_ref());
    }
    Ok(IpcRequest {
        op: op.to_string(),
        params: Value::Object(params),
    })
}

fn parse_query_params(
    query: &str,
) -> Result<Map<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
    let mut params = Map::new();
    if query.is_empty() {
        return Ok(params);
    }
    let scratch_base = Url::parse("http://loadlynx.invalid/")?;
    let url = scratch_base.join(&format!("?{query}"))?;
    for (key, value) in url.query_pairs() {
        let value = value.into_owned();
        let value = value
            .parse::<usize>()
            .map(|number| json!(number))
            .unwrap_or_else(|_| json!(value));
        params.insert(key.into_owned(), value);
    }
    Ok(params)
}

fn merge_body_object(
    params: &mut Map<String, Value>,
    body: Option<Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(body) = body else {
        return Ok(());
    };
    let Some(object) = body.as_object() else {
        return Err("devd IPC route requires an object body".into());
    };
    params.extend(object.clone());
    Ok(())
}

fn set_body(params: &mut Map<String, Value>, body: Option<&Value>) {
    if let Some(body) = body {
        params.insert("body".to_string(), body.clone());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let json_output = cli.json;
    let devd = cli.ipc;
    let allow_interactive = !json_output && io::stdin().is_terminal() && io::stdout().is_terminal();
    for endpoint in initial_devd_endpoints(&cli.command, &devd) {
        ensure_ipc_devd(&endpoint, !cli.no_auto_start).await?;
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let payload_result: Result<Value, Box<dyn std::error::Error + Send + Sync>> = async {
        let payload = match cli.command {
            Command::Completion { shell } => {
                let mut cmd = Cli::command();
                generate(shell, &mut cmd, "loadlynx", &mut io::stdout());
                json!({"__loadlynx_cli_already_printed": true})
            }
            Command::Devices => {
                handle_device_command(DeviceCommand::List, &client, &devd, allow_interactive)
                    .await?
            }
            Command::Device { command } => {
                handle_device_command(command, &client, &devd, allow_interactive).await?
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
                let scan =
                    request_devd_value(&devd, reqwest::Method::POST, "/api/v1/devices/scan", None)
                        .await?;
                json!({"mdns_requested": mdns, "lan_scan_requested": lan_scan, "devd": scan})
            }
            Command::Status { url, device } => {
                ensure_one_status_selector(url.as_ref(), device.as_ref())?;
                if let Some(url) = url {
                    client
                        .get(api_url(&url, "/api/v1/status")?)
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                } else {
                    match resolve_saved_hardware_selection(device, &devd, allow_interactive)? {
                        ResolvedHardware::Usb(resolved) => {
                            let status = request_devd_usb_value(
                                &client,
                                &resolved,
                                reqwest::Method::GET,
                                "/api/v1/status",
                                None,
                            )
                            .await?;
                            let _ = mark_hardware_transport_used(
                                &resolved.hardware_id,
                                SavedTransport::Usb,
                            );
                            status
                        }
                        ResolvedHardware::Http { hardware_id, url } => {
                            let status = client
                                .get(api_url(&url, "/api/v1/status")?)
                                .send()
                                .await?
                                .error_for_status()?
                                .json::<Value>()
                                .await?;
                            let _ =
                                mark_hardware_transport_used(&hardware_id, SavedTransport::Http);
                            status
                        }
                    }
                }
            }
            Command::Flash {
                target,
                device,
                artifact,
                manifest_path,
                dry_run,
                confirm,
                expected_identity_device_id,
                acknowledge_non_project_firmware,
            } => {
                let resolved = resolve_usb_target(device, &devd, allow_interactive)?;
                let resolved = ResolvedUsbHardware {
                    expected_identity_device_id: expected_identity_device_id
                        .clone()
                        .or(resolved.expected_identity_device_id),
                    ..resolved
                };
                if manifest_path.is_some() {
                    select_device_artifact(
                        &client,
                        &resolved,
                        manifest_path.clone(),
                        artifact.clone(),
                    )
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
                        "expected_identity_device_id": resolved.expected_identity_device_id,
                        "acknowledge_non_project_firmware": acknowledge_non_project_firmware,
                    }),
                    dry_run,
                )
                .await?
            }
            Command::Reset {
                target,
                device,
                dry_run,
            } => {
                let resolved = resolve_usb_target(device, &devd, allow_interactive)?;
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
                tail,
                format,
            } => {
                let resolved = resolve_usb_target(device, &devd, allow_interactive)?;
                run_monitor(&client, resolved, tail, format).await?
            }
            Command::Cc {
                target_i_ma,
                url,
                device,
                preset_id,
                min_v_mv,
                max_i_ma_total,
                max_p_mw,
                disable,
            } => {
                handle_mode_first_command(
                    &client,
                    &devd,
                    ModeFirstCommand::Cc,
                    target_i_ma,
                    None,
                    None,
                    url,
                    device,
                    allow_interactive,
                    preset_id,
                    min_v_mv,
                    max_i_ma_total,
                    max_p_mw,
                    disable,
                )
                .await?
            }
            Command::Cv {
                target_v_mv,
                url,
                device,
                preset_id,
                min_v_mv,
                max_i_ma_total,
                max_p_mw,
                disable,
            } => {
                handle_mode_first_command(
                    &client,
                    &devd,
                    ModeFirstCommand::Cv,
                    0,
                    Some(target_v_mv),
                    None,
                    url,
                    device,
                    allow_interactive,
                    preset_id,
                    min_v_mv,
                    max_i_ma_total,
                    max_p_mw,
                    disable,
                )
                .await?
            }
            Command::Cp {
                target_p_mw,
                url,
                device,
                preset_id,
                min_v_mv,
                max_i_ma_total,
                max_p_mw,
                disable,
            } => {
                handle_mode_first_command(
                    &client,
                    &devd,
                    ModeFirstCommand::Cp,
                    0,
                    None,
                    Some(target_p_mw),
                    url,
                    device,
                    allow_interactive,
                    preset_id,
                    min_v_mv,
                    max_i_ma_total,
                    max_p_mw,
                    disable,
                )
                .await?
            }
            Command::Pd { command } => match command {
                PdCommand::Set {
                    device,
                    mode,
                    object_pos,
                    target_mv,
                    i_req_ma,
                    allow_extended_voltage,
                } => {
                    let resolved = resolve_usb_target(device, &devd, allow_interactive)?;
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
                WifiCommand::Show { url, device } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
                    ssid,
                    psk,
                    wait,
                    allow_insecure_lan_wifi,
                } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
                    allow_insecure_lan_wifi,
                } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::DELETE,
                        "/api/v1/wifi",
                        None,
                        allow_insecure_lan_wifi,
                    )
                    .await?
                }
            },
            Command::Control { command } => match command {
                ControlCommand::Get { url, device } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
                    enable,
                    disable,
                } => {
                    let output_enabled = resolve_output_enable(enable, disable)?;
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::POST,
                        "/api/v1/control",
                        Some(json!({"output_enabled": output_enabled})),
                        false,
                    )
                    .await?
                }
            },
            Command::Preset { command } => match command {
                PresetCommand::List { url, device } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::GET,
                        "/api/v1/presets",
                        None,
                        false,
                    )
                    .await?
                }
                PresetCommand::Set { url, device, file } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
                    preset_id,
                } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::POST,
                        "/api/v1/presets/apply",
                        Some(json!({"preset_id": preset_id})),
                        false,
                    )
                    .await?
                }
            },
            Command::Calibration { command } => match command {
                CalibrationCommand::Profile { url, device } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::GET,
                        "/api/v1/calibration/profile",
                        None,
                        false,
                    )
                    .await?
                }
                CalibrationCommand::Mode { url, device, kind } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::POST,
                        "/api/v1/calibration/mode",
                        Some(json!({"kind": kind})),
                        false,
                    )
                    .await?
                }
                CalibrationCommand::Apply { url, device, file } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::POST,
                        "/api/v1/calibration/apply",
                        Some(read_json_file(&file)?),
                        false,
                    )
                    .await?
                }
                CalibrationCommand::Commit { url, device, file } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        reqwest::Method::POST,
                        "/api/v1/calibration/commit",
                        Some(read_json_file(&file)?),
                        false,
                    )
                    .await?
                }
                CalibrationCommand::Reset { url, device, kind } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
                reason,
            } => {
                request_api_value(
                    &client,
                    &devd,
                    ApiSelector { url, device },
                    allow_interactive,
                    reqwest::Method::POST,
                    "/api/v1/soft-reset",
                    Some(json!({"reason": reason})),
                    false,
                )
                .await?
            }
            Command::Diagnostics { command } => match command {
                DiagnosticsCommand::Export { url, device } => {
                    request_api_value(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
                    file,
                    include,
                } => {
                    handle_backup_export(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
                        &file,
                        &include,
                    )
                    .await?
                }
                BackupCommand::Import {
                    url,
                    device,
                    file,
                    include,
                    dry_run,
                    allow_insecure_lan_wifi,
                } => {
                    handle_backup_import(
                        &client,
                        &devd,
                        ApiSelector { url, device },
                        allow_interactive,
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
    let body = json!({"manifest_path": manifest_path, "artifact_id": artifact_id});
    let result = request_devd_value(
        &resolved.devd,
        reqwest::Method::POST,
        &format!("/api/v1/devices/{}/artifact", resolved.device),
        Some(body.clone()),
    )
    .await;
    match result {
        Ok(value) => Ok(value),
        Err(error) if saved_usb_device_needs_relookup(&*error) => {
            let scan = request_devd_value(
                &resolved.devd,
                reqwest::Method::POST,
                "/api/v1/devices/scan",
                None,
            )
            .await?;
            let device = resolve_scanned_usb_device_for_saved_hardware(resolved, &scan)?;
            request_devd_value(
                &resolved.devd,
                reqwest::Method::POST,
                &format!("/api/v1/devices/{device}/artifact"),
                Some(body),
            )
            .await
        }
        Err(error) => Err(error),
    }
}

fn initial_devd_endpoints(command: &Command, default_devd: &str) -> Vec<String> {
    let endpoints = match command {
        Command::Completion { .. } => Vec::new(),
        Command::Device { command } => match command {
            DeviceCommand::List | DeviceCommand::Use { .. } | DeviceCommand::Remove { .. } => {
                Vec::new()
            }
            DeviceCommand::Add { url, .. } => {
                if url.is_some() {
                    Vec::new()
                } else {
                    vec![default_devd.to_string()]
                }
            }
        },
        Command::Discover { .. } => vec![default_devd.to_string()],
        Command::Devices => Vec::new(),
        Command::Status { url, device } => {
            selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd)
                .into_iter()
                .collect()
        }
        Command::Flash { device, .. }
        | Command::Reset { device, .. }
        | Command::Monitor { device, .. }
        | Command::Pd {
            command: PdCommand::Set { device, .. },
        } => usb_target_devd_endpoint(device.as_ref(), default_devd)
            .into_iter()
            .collect(),
        Command::Cc { url, device, .. }
        | Command::Cv { url, device, .. }
        | Command::Cp { url, device, .. } => {
            selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd)
                .into_iter()
                .collect()
        }
        Command::Wifi { command } => match command {
            WifiCommand::Show { url, device }
            | WifiCommand::Set { url, device, .. }
            | WifiCommand::Clear { url, device, .. } => {
                selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd)
                    .into_iter()
                    .collect()
            }
        },
        Command::Control { command } => match command {
            ControlCommand::Get { url, device } | ControlCommand::Set { url, device, .. } => {
                selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd)
                    .into_iter()
                    .collect()
            }
        },
        Command::Preset { command } => match command {
            PresetCommand::List { url, device }
            | PresetCommand::Set { url, device, .. }
            | PresetCommand::Apply { url, device, .. } => {
                { selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd) }
                    .into_iter()
                    .collect()
            }
        },
        Command::Calibration { command } => match command {
            CalibrationCommand::Profile { url, device }
            | CalibrationCommand::Mode { url, device, .. }
            | CalibrationCommand::Apply { url, device, .. }
            | CalibrationCommand::Commit { url, device, .. }
            | CalibrationCommand::Reset { url, device, .. } => {
                selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd)
                    .into_iter()
                    .collect()
            }
        },
        Command::SoftReset { url, device, .. }
        | Command::Diagnostics {
            command: DiagnosticsCommand::Export { url, device },
        } => selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd)
            .into_iter()
            .collect(),
        Command::Backup { command } => match command {
            BackupCommand::Export { url, device, .. }
            | BackupCommand::Import { url, device, .. } => {
                { selector_devd_endpoint(url.as_ref(), device.as_ref(), default_devd) }
                    .into_iter()
                    .collect()
            }
        },
        Command::UsbPort { .. } => Vec::new(),
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
    default_devd: &str,
) -> Option<String> {
    if url.is_some() {
        return None;
    }
    let resolved = resolve_saved_hardware_selection(device.cloned(), default_devd, false).ok();
    let endpoint = resolved.and_then(|resolved| match resolved {
        ResolvedHardware::Usb(resolved) => Some(resolved.devd),
        ResolvedHardware::Http { .. } => None,
    });
    endpoint.or_else(|| {
        has_saved_device_for_transport(Some(SavedTransport::Usb))
            .ok()
            .filter(|has_usb| *has_usb)
            .map(|_| default_devd.to_string())
    })
}

fn usb_target_devd_endpoint(device: Option<&String>, default_devd: &str) -> Option<String> {
    let resolved = resolve_saved_hardware_selection_with_transport(
        device.cloned(),
        default_devd,
        false,
        Some(SavedTransport::Usb),
    )
    .ok();
    let endpoint = resolved.and_then(|resolved| match resolved {
        ResolvedHardware::Usb(resolved) => Some(resolved.devd),
        ResolvedHardware::Http { .. } => None,
    });
    endpoint.or_else(|| {
        has_saved_device_for_transport(Some(SavedTransport::Usb))
            .ok()
            .filter(|has_usb| *has_usb)
            .map(|_| default_devd.to_string())
    })
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

fn read_json_file(path: &Path) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        extract::State,
        routing::{get, post},
    };
    use loadlynx_devd::IpcResponse;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::sync::{LazyLock, Mutex as StdMutex};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    static TEST_ENV_LOCK: LazyLock<StdMutex<()>> = LazyLock::new(|| StdMutex::new(()));

    #[derive(Clone, Default)]
    struct TestHttpState {
        lease_creates: Arc<AtomicUsize>,
        scans: Arc<AtomicUsize>,
        operation_payloads: Arc<Mutex<Vec<Value>>>,
        lease_payloads: Arc<Mutex<Vec<Value>>>,
        artifact_devices: Arc<Mutex<Vec<String>>>,
    }

    #[derive(Clone, Default)]
    struct ModeFirstHttpState {
        identity_gets: Arc<AtomicUsize>,
        cc_posts: Arc<AtomicUsize>,
        control_posts: Arc<AtomicUsize>,
        presets_gets: Arc<AtomicUsize>,
        cc_payloads: Arc<Mutex<Vec<Value>>>,
    }

    fn mode_first_control_payload(output_enabled: bool) -> Value {
        json!({
            "active_preset_id": 1,
            "output_enabled": output_enabled,
            "uv_latched": false,
            "preset": {
                "preset_id": 1,
                "mode": "cc",
                "target_i_ma": 1234,
                "target_v_mv": 12000,
                "target_p_mw": 15000,
                "min_v_mv": 0,
                "max_i_ma_total": 10000,
                "max_p_mw": 150000
            }
        })
    }

    async fn spawn_legacy_mode_first_http(state: ModeFirstHttpState) -> String {
        async fn identity(State(state): State<ModeFirstHttpState>) -> axum::Json<Value> {
            state.identity_gets.fetch_add(1, Ordering::SeqCst);
            axum::Json(json!({
                "device_id": "loadlynx-legacy",
                "capabilities": {
                    "cc_supported": true,
                    "cv_supported": false,
                    "cp_supported": false,
                    "presets_supported": false
                }
            }))
        }

        async fn post_cc(
            State(state): State<ModeFirstHttpState>,
            axum::Json(payload): axum::Json<Value>,
        ) -> axum::Json<Value> {
            let enable = payload
                .get("enable")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            state.cc_posts.fetch_add(1, Ordering::SeqCst);
            state
                .cc_payloads
                .lock()
                .expect("cc payloads lock")
                .push(payload);
            axum::Json(json!({
                "ok": true,
                "request_id": "request-1",
                "response": {
                    "request_id": "request-1",
                    "data": {
                        "enable": enable,
                        "target_i_ma": 2000
                    }
                }
            }))
        }

        async fn post_control(
            State(state): State<ModeFirstHttpState>,
            axum::Json(_payload): axum::Json<Value>,
        ) -> axum::Json<Value> {
            state.control_posts.fetch_add(1, Ordering::SeqCst);
            axum::Json(mode_first_control_payload(false))
        }

        async fn presets(State(state): State<ModeFirstHttpState>) -> axum::Json<Value> {
            state.presets_gets.fetch_add(1, Ordering::SeqCst);
            axum::Json(json!({"presets": []}))
        }

        let app = Router::new()
            .route("/api/v1/identity", get(identity))
            .route("/api/v1/cc", post(post_cc))
            .route("/api/v1/control", post(post_control))
            .route("/api/v1/presets", get(presets))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn spawn_test_ipc(state: TestHttpState) -> String {
        spawn_test_ipc_with_mode(state, TestIpcMode::Normal).await
    }

    async fn spawn_scan_required_test_ipc(state: TestHttpState) -> String {
        spawn_test_ipc_with_mode(state, TestIpcMode::ScanRequiredLease).await
    }

    async fn spawn_artifact_scan_required_test_ipc(state: TestHttpState) -> String {
        spawn_test_ipc_with_mode(state, TestIpcMode::ScanRequiredArtifact).await
    }

    async fn spawn_identity_mismatch_then_scan_test_ipc(state: TestHttpState) -> String {
        spawn_test_ipc_with_mode(state, TestIpcMode::IdentityMismatchThenScan).await
    }

    #[derive(Clone, Copy)]
    enum TestIpcMode {
        Normal,
        ScanRequiredLease,
        ScanRequiredArtifact,
        IdentityMismatchThenScan,
    }

    async fn spawn_test_ipc_with_mode(state: TestHttpState, mode: TestIpcMode) -> String {
        let temp_dir = tempfile::tempdir().expect("temp ipc dir");
        let endpoint = temp_dir.path().join("loadlynx.sock");
        let endpoint_string = endpoint.to_string_lossy().to_string();
        let listener = tokio::net::UnixListener::bind(&endpoint).expect("bind test IPC");
        tokio::spawn(async move {
            let _keep_dir_alive = temp_dir;
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let state = state.clone();
                tokio::spawn(async move {
                    let (reader, mut writer) = tokio::io::split(stream);
                    let mut lines = tokio::io::BufReader::new(reader).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let response = match serde_json::from_str::<IpcRequest>(&line) {
                            Ok(request) => handle_test_ipc_request(&state, mode, request),
                            Err(error) => IpcResponse {
                                ok: false,
                                result: None,
                                error: Some(loadlynx_devd::ApiError {
                                    code: "ipc_invalid_json".to_string(),
                                    message: error.to_string(),
                                    retryable: false,
                                    details: None,
                                }),
                            },
                        };
                        let text = serde_json::to_string(&response).expect("test IPC response");
                        if writer.write_all(text.as_bytes()).await.is_err()
                            || writer.write_all(b"\n").await.is_err()
                        {
                            break;
                        }
                    }
                });
            }
        });
        endpoint_string
    }

    fn handle_test_ipc_request(
        state: &TestHttpState,
        mode: TestIpcMode,
        request: IpcRequest,
    ) -> IpcResponse {
        let params = request.params;
        let result = match request.op.as_str() {
            "serial.lease.create" => test_create_lease(state, mode, params),
            "serial.lease.heartbeat" | "serial.lease.release" => Ok(json!({"ok": true})),
            "compat.status" => Ok(json!({"ok": true, "link_up": true})),
            "devices.scan" => {
                state.scans.fetch_add(1, Ordering::SeqCst);
                let id = match mode {
                    TestIpcMode::Normal => "digital-1",
                    TestIpcMode::ScanRequiredLease
                    | TestIpcMode::ScanRequiredArtifact
                    | TestIpcMode::IdentityMismatchThenScan => "digital-current",
                };
                Ok(
                    json!({"devices": [{"id": id, "digital_target": {"port_path": "mock://esp32s3"}}]}),
                )
            }
            "devices.flash" | "devices.reset" => {
                state
                    .operation_payloads
                    .lock()
                    .expect("operation payloads lock")
                    .push(params.clone());
                Ok(json!({"ok": true, "payload": params}))
            }
            "devices.artifact.select" => test_select_artifact(state, mode, params),
            _ => Err((
                "ipc_unknown_operation",
                format!("unknown op {}", request.op),
            )),
        };

        match result {
            Ok(value) => IpcResponse {
                ok: true,
                result: Some(value),
                error: None,
            },
            Err((code, message)) => IpcResponse {
                ok: false,
                result: None,
                error: Some(loadlynx_devd::ApiError {
                    code: code.to_string(),
                    message,
                    retryable: false,
                    details: None,
                }),
            },
        }
    }

    fn test_create_lease(
        state: &TestHttpState,
        mode: TestIpcMode,
        payload: Value,
    ) -> Result<Value, (&'static str, String)> {
        state.lease_creates.fetch_add(1, Ordering::SeqCst);
        state
            .lease_payloads
            .lock()
            .expect("lease payloads lock")
            .push(payload.clone());
        match mode {
            TestIpcMode::ScanRequiredLease if state.scans.load(Ordering::SeqCst) == 0 => {
                Err(("device_not_found", "device is not known".to_string()))
            }
            TestIpcMode::IdentityMismatchThenScan if state.scans.load(Ordering::SeqCst) == 0 => {
                Err((
                    "identity_confirmation_mismatch",
                    "identity confirmation mismatch".to_string(),
                ))
            }
            _ => Ok(json!({
                "lease_id": "lease-1",
                "identity_device_id": payload.get("expected_identity_device_id").cloned().unwrap_or(Value::Null),
                "heartbeat_interval_ms": 1000
            })),
        }
    }

    fn test_select_artifact(
        state: &TestHttpState,
        mode: TestIpcMode,
        payload: Value,
    ) -> Result<Value, (&'static str, String)> {
        let device = payload
            .get("device_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>")
            .to_string();
        state
            .artifact_devices
            .lock()
            .expect("artifact devices lock")
            .push(device.clone());
        if matches!(mode, TestIpcMode::ScanRequiredArtifact)
            && state.scans.load(Ordering::SeqCst) == 0
        {
            return Err(("device_not_found", "device is not known".to_string()));
        }
        Ok(json!({"ok": true, "device_id": device, "payload": payload}))
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
            },
            false,
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
            },
            false,
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
        };
        let selection = parse_backup_selection(&[]).expect("default selection");

        let err = preflight_backup_restore(
            "http://127.0.0.1:9",
            &selector,
            false,
            &backup,
            selection,
            false,
        )
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

        let cli =
            Cli::try_parse_from(["loadlynx", "--ipc", "/tmp/loadlynx.sock", "device", "list"])
                .expect("device list parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());

        let cli = Cli::try_parse_from(["loadlynx", "--ipc", "/tmp/loadlynx.sock", "devices"])
            .expect("devices parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());
    }

    #[test]
    fn initial_devd_endpoints_include_usb_commands() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let previous_home = env::var_os("LOADLYNX_HOME");
        // Tests serialize environment mutation through TEST_ENV_LOCK.
        unsafe { env::set_var("LOADLYNX_HOME", temp.path()) };
        write_hardware_registry(
            &temp.path().join("devices.json"),
            &HardwareRegistry {
                default_hardware_id: Some("loadlynx-a1b2c3".to_string()),
                hardware: vec![SavedHardware {
                    id: "loadlynx-a1b2c3".to_string(),
                    name: None,
                    identity: None,
                    last_transport: Some(SavedTransport::Usb),
                    transports: SavedTransports {
                        usb: Some(SavedUsbTransport {
                            device: "digital-1".to_string(),
                            port_path: Some("mock://esp32s3".to_string()),
                            devd: None,
                        }),
                        http: None,
                    },
                    last_seen_unix_seconds: None,
                }],
                ..HardwareRegistry::default()
            },
        )
        .unwrap();

        let cli = Cli::try_parse_from(["loadlynx", "--ipc", "/tmp/loadlynx.sock", "devices"])
            .expect("devices parse");
        assert!(initial_devd_endpoints(&cli.command, &cli.ipc).is_empty());

        let cli = Cli::try_parse_from([
            "loadlynx",
            "--ipc",
            "/tmp/loadlynx.sock",
            "status",
            "--device",
            "loadlynx-a1b2c3",
        ])
        .expect("status device parse");
        assert_eq!(
            initial_devd_endpoints(&cli.command, &cli.ipc),
            vec!["/tmp/loadlynx.sock"]
        );

        let cli = Cli::try_parse_from(["loadlynx", "--ipc", "/tmp/loadlynx.sock", "device", "add"])
            .expect("device add parse");
        assert_eq!(
            initial_devd_endpoints(&cli.command, &cli.ipc),
            vec!["/tmp/loadlynx.sock"]
        );

        match previous_home {
            Some(value) => unsafe { env::set_var("LOADLYNX_HOME", value) },
            None => unsafe { env::remove_var("LOADLYNX_HOME") },
        }
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

    #[test]
    fn ipc_request_for_devd_call_maps_compat_status_to_native_operation() {
        let request = ipc_request_for_devd_call(
            reqwest::Method::GET,
            "/api/v1/status?device_id=loadlynx-a1b2c3&lease_id=lease-1",
            None,
        )
        .expect("native status IPC request");

        assert_eq!(request.op, "compat.status");
        assert_eq!(
            request.params.get("device_id").and_then(Value::as_str),
            Some("loadlynx-a1b2c3")
        );
        assert_eq!(
            request.params.get("lease_id").and_then(Value::as_str),
            Some("lease-1")
        );
    }

    #[tokio::test]
    async fn request_devd_value_rejects_http_devd_endpoint() {
        let error = request_devd_value(
            "http://127.0.0.1:30180",
            reqwest::Method::POST,
            "/api/v1/devices/digital-1/reset",
            Some(json!({"dry_run": true})),
        )
        .await
        .expect_err("HTTP devd endpoint must be rejected");

        assert!(
            error
                .to_string()
                .contains("devd endpoint must be a native IPC endpoint")
        );
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
        let devd = spawn_test_ipc(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "mock-loadlynx-devd".to_string(),
            device: "digital-1".to_string(),
            devd,
            port_path: None,
            expected_identity_device_id: Some("mock-loadlynx-devd".to_string()),
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
    async fn real_usb_firmware_operation_creates_preflash_lease() {
        let state = TestHttpState::default();
        let devd = spawn_test_ipc(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "mock-loadlynx-devd".to_string(),
            device: "digital-1".to_string(),
            devd,
            port_path: None,
            expected_identity_device_id: Some("mock-loadlynx-devd".to_string()),
        };

        post_usb_operation_with_optional_lease(
            &Client::new(),
            &resolved,
            "/api/v1/devices/digital-1/flash",
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
        let lease_payloads = state.lease_payloads.lock().expect("lease payloads lock");
        assert_eq!(
            lease_payloads[0]
                .get("allow_legacy_preflash_identity_fallback")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[tokio::test]
    async fn real_usb_reset_operation_creates_strict_cli_lease() {
        let state = TestHttpState::default();
        let devd = spawn_test_ipc(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "mock-loadlynx-devd".to_string(),
            device: "digital-1".to_string(),
            devd,
            port_path: None,
            expected_identity_device_id: Some("mock-loadlynx-devd".to_string()),
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

        let lease_payloads = state.lease_payloads.lock().expect("lease payloads lock");
        assert!(
            lease_payloads[0]
                .get("allow_legacy_preflash_identity_fallback")
                .is_none()
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
        let mut registry = HardwareRegistry {
            default_hardware_id: Some("loadlynx-a1b2c3".to_string()),
            ..HardwareRegistry::default()
        };
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-a1b2c3".to_string(),
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
            "loadlynx-a1b2c3".to_string(),
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
            Some("loadlynx-a1b2c3")
        );
        assert_eq!(reloaded.hardware.len(), 1);
        assert_eq!(reloaded.hardware[0].id, "loadlynx-a1b2c3");
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
                    "id": "loadlynx-a1b2c3",
                    "name": "Bench",
                    "transport": "usb",
                    "device": "digital-1",
                    "devd": "http://127.0.0.1:30180",
                    "last_seen_unix_seconds": 10
                },
                {
                    "id": "loadlynx-a1b2c3",
                    "transport": "http",
                    "url": "http://loadlynx-bench.local",
                    "last_seen_unix_seconds": 20
                }
            ]
        });

        let migrated = migrate_legacy_hardware_registry(legacy).unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.hardware.len(), 1);
        assert_eq!(migrated.hardware[0].id, "loadlynx-a1b2c3");
        assert_eq!(
            migrated.hardware[0].last_transport,
            Some(SavedTransport::Http)
        );
        assert!(migrated.hardware[0].transports.usb.is_some());
        assert!(migrated.hardware[0].transports.http.is_some());
    }

    #[test]
    fn legacy_usb_hardware_does_not_expect_old_generated_id() {
        let legacy = SavedHardware {
            id: "usb-digital-1".to_string(),
            name: None,
            identity: None,
            last_transport: Some(SavedTransport::Usb),
            transports: SavedTransports {
                usb: Some(SavedUsbTransport {
                    device: "digital-1".to_string(),
                    port_path: Some("mock://esp32s3".to_string()),
                    devd: None,
                }),
                http: None,
            },
            last_seen_unix_seconds: None,
        };
        let stable = SavedHardware {
            id: "loadlynx-abc123".to_string(),
            name: None,
            identity: None,
            last_transport: Some(SavedTransport::Usb),
            transports: legacy.transports.clone(),
            last_seen_unix_seconds: None,
        };

        match resolve_hardware_transport(&legacy, SavedTransport::Usb, "http://devd").unwrap() {
            ResolvedHardware::Usb(resolved) => {
                assert!(resolved.expected_identity_device_id.is_none())
            }
            ResolvedHardware::Http { .. } => panic!("expected usb hardware"),
        }
        match resolve_hardware_transport(&stable, SavedTransport::Usb, "http://devd").unwrap() {
            ResolvedHardware::Usb(resolved) => assert_eq!(
                resolved.expected_identity_device_id.as_deref(),
                Some("loadlynx-abc123")
            ),
            ResolvedHardware::Http { .. } => panic!("expected usb hardware"),
        }
    }

    #[test]
    fn resolved_http_hardware_carries_real_hardware_id() {
        let hardware = SavedHardware {
            id: "loadlynx-d4e5f6".to_string(),
            name: None,
            identity: None,
            last_transport: Some(SavedTransport::Http),
            transports: SavedTransports {
                usb: None,
                http: Some(SavedHttpTransport {
                    url: "http://loadlynx-http.local".to_string(),
                }),
            },
            last_seen_unix_seconds: None,
        };

        match resolve_hardware_transport(&hardware, SavedTransport::Http, "http://devd").unwrap() {
            ResolvedHardware::Http { hardware_id, url } => {
                assert_eq!(hardware_id, "loadlynx-d4e5f6");
                assert_eq!(url, "http://loadlynx-http.local");
            }
            ResolvedHardware::Usb(_) => panic!("expected http hardware"),
        }
    }

    #[test]
    fn device_commands_parse_saved_device_workflows() {
        let cli = Cli::try_parse_from(["loadlynx", "device", "add", "--name", "Bench"]).unwrap();
        match cli.command {
            Command::Device {
                command: DeviceCommand::Add { url, name },
            } => {
                assert!(url.is_none());
                assert_eq!(name.as_deref(), Some("Bench"));
            }
            _ => panic!("expected device add command"),
        }

        let cli =
            Cli::try_parse_from(["loadlynx", "status", "--device", "loadlynx-abc123"]).unwrap();
        match cli.command {
            Command::Status { device, .. } => {
                assert_eq!(device.as_deref(), Some("loadlynx-abc123"));
            }
            _ => panic!("expected status command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "devices"]).unwrap();
        match cli.command {
            Command::Devices => {}
            _ => panic!("expected devices command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "device", "use", "--global", "loadlynx-abc123"])
            .unwrap();
        match cli.command {
            Command::Device {
                command: DeviceCommand::Use { id, global, clear },
            } => {
                assert_eq!(id.as_deref(), Some("loadlynx-abc123"));
                assert!(global);
                assert!(!clear);
            }
            _ => panic!("expected device use command"),
        }

        let cli = Cli::try_parse_from([
            "loadlynx",
            "cc",
            "2000",
            "--device",
            "loadlynx-abc123",
            "--preset-id",
            "2",
            "--disable",
        ])
        .unwrap();
        match cli.command {
            Command::Cc {
                target_i_ma,
                device,
                preset_id,
                disable,
                ..
            } => {
                assert_eq!(target_i_ma, 2000);
                assert_eq!(device.as_deref(), Some("loadlynx-abc123"));
                assert_eq!(preset_id, Some(2));
                assert!(disable);
            }
            _ => panic!("expected cc command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "cc", "2000", "--url", "http://127.0.0.1:9100"])
            .unwrap();
        match cli.command {
            Command::Cc {
                target_i_ma, url, ..
            } => {
                assert_eq!(target_i_ma, 2000);
                assert_eq!(url.as_deref(), Some("http://127.0.0.1:9100"));
            }
            _ => panic!("expected cc command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "cv", "24500", "--device", "loadlynx-abc123"])
            .unwrap();
        match cli.command {
            Command::Cv {
                target_v_mv,
                device,
                ..
            } => {
                assert_eq!(target_v_mv, 24_500);
                assert_eq!(device.as_deref(), Some("loadlynx-abc123"));
            }
            _ => panic!("expected cv command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "cp", "60000", "--device", "loadlynx-abc123"])
            .unwrap();
        match cli.command {
            Command::Cp {
                target_p_mw,
                device,
                ..
            } => {
                assert_eq!(target_p_mw, 60_000);
                assert_eq!(device.as_deref(), Some("loadlynx-abc123"));
            }
            _ => panic!("expected cp command"),
        }

        let cli = Cli::try_parse_from([
            "loadlynx",
            "control",
            "set",
            "--device",
            "loadlynx-abc123",
            "--enable",
        ])
        .unwrap();
        match cli.command {
            Command::Control {
                command:
                    ControlCommand::Set {
                        device,
                        enable,
                        disable,
                        ..
                    },
            } => {
                assert_eq!(device.as_deref(), Some("loadlynx-abc123"));
                assert!(enable);
                assert!(!disable);
            }
            _ => panic!("expected control set command"),
        }
    }

    #[test]
    fn mode_first_commands_parse_and_validate_targets() {
        assert!(resolve_output_enable(true, false).unwrap());
        assert!(!resolve_output_enable(false, true).unwrap());
        assert!(resolve_output_enable(true, true).is_err());
        assert!(resolve_output_enable(false, false).is_err());
    }

    #[tokio::test]
    async fn mode_first_legacy_cc_uses_cc_endpoint_without_presets() {
        let state = ModeFirstHttpState::default();
        let url = spawn_legacy_mode_first_http(state.clone()).await;

        let value = handle_mode_first_command_for_selector(
            &Client::new(),
            "http://127.0.0.1:0",
            ApiSelector {
                url: Some(url.clone()),
                device: None,
            },
            false,
            ModeFirstCommand::Cc,
            2_000,
            None,
            None,
            Some(2),
            None,
            None,
            None,
            false,
        )
        .await
        .expect("legacy cc should use compatibility endpoint");

        assert_eq!(value.get("mode").and_then(Value::as_str), Some("CC"));
        assert_eq!(
            value.get("output_enabled").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            value.get("target_i_ma").and_then(Value::as_u64),
            Some(2_000)
        );
        assert_eq!(
            value
                .pointer("/cc/response/data/enable")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(state.identity_gets.load(Ordering::SeqCst), 1);
        assert_eq!(state.cc_posts.load(Ordering::SeqCst), 1);
        assert_eq!(state.control_posts.load(Ordering::SeqCst), 0);
        assert_eq!(state.presets_gets.load(Ordering::SeqCst), 0);

        let disable_value = handle_mode_first_command_for_selector(
            &Client::new(),
            "http://127.0.0.1:0",
            ApiSelector {
                url: Some(url),
                device: None,
            },
            false,
            ModeFirstCommand::Cc,
            2_000,
            None,
            None,
            Some(2),
            None,
            None,
            None,
            true,
        )
        .await
        .expect("legacy cc disable should use compatibility endpoint");

        assert_eq!(
            disable_value.get("output_enabled").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            disable_value
                .pointer("/cc/response/data/enable")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(state.identity_gets.load(Ordering::SeqCst), 2);
        assert_eq!(state.cc_posts.load(Ordering::SeqCst), 2);
        assert_eq!(state.control_posts.load(Ordering::SeqCst), 0);
        assert_eq!(state.presets_gets.load(Ordering::SeqCst), 0);

        let payloads = state.cc_payloads.lock().expect("cc payloads lock");
        assert_eq!(payloads.len(), 2);
        assert_eq!(
            payloads[0].get("target_i_ma").and_then(Value::as_u64),
            Some(2_000)
        );
        assert_eq!(
            payloads[0].get("enable").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payloads[1].get("enable").and_then(Value::as_bool),
            Some(false)
        );
        assert!(payloads[1].get("target_i_ma").is_none());
    }

    #[test]
    fn cli_errors_are_classified_for_json_automation() {
        assert_eq!(
            classify_cli_error_code(
                "default device is not set; run `loadlynx device use --global <saved-id>`"
            ),
            "default_device_not_set"
        );
        assert_eq!(
            classify_cli_error_code(
                "identity device_id `digital-esp32s3` is not a stable LoadLynx hardware id"
            ),
            "unstable_hardware_identity"
        );
    }

    #[test]
    fn stable_hardware_ids_require_mac_derived_shape() {
        assert!(is_stable_hardware_id("loadlynx-a1b2c3"));
        assert!(is_stable_hardware_id("loadlynx-012345"));
        assert!(is_stable_hardware_id("mock-loadlynx-devd"));
        assert!(!is_stable_hardware_id("digital-esp32s3"));
        assert!(!is_stable_hardware_id("loadlynx-bench"));
        assert!(!is_stable_hardware_id("loadlynx-A1B2C3"));
        assert!(!is_stable_hardware_id("llx-digital-01"));
    }

    #[tokio::test]
    async fn bind_probe_lease_marks_explicit_binding_probe() {
        let state = TestHttpState::default();
        let devd = spawn_test_ipc(state.clone()).await;

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
        let devd = spawn_scan_required_test_ipc(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "loadlynx-a1b2c3".to_string(),
            device: "digital-stale".to_string(),
            devd,
            port_path: Some("mock://esp32s3".to_string()),
            expected_identity_device_id: Some("loadlynx-a1b2c3".to_string()),
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
            Some("loadlynx-a1b2c3")
        );
    }

    #[tokio::test]
    async fn saved_usb_request_scans_after_identity_mismatch() {
        let state = TestHttpState::default();
        let devd = spawn_identity_mismatch_then_scan_test_ipc(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "loadlynx-a1b2c3".to_string(),
            device: "digital-reused".to_string(),
            devd,
            port_path: Some("mock://esp32s3".to_string()),
            expected_identity_device_id: Some("loadlynx-a1b2c3".to_string()),
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
        let payloads = state.lease_payloads.lock().expect("lease payloads lock");
        assert_eq!(
            payloads[1].get("device_id").and_then(Value::as_str),
            Some("digital-current")
        );
        assert_eq!(
            payloads[1]
                .get("expected_identity_device_id")
                .and_then(Value::as_str),
            Some("loadlynx-a1b2c3")
        );
    }

    #[test]
    fn legacy_usb_lease_rejects_unstable_identity_when_reported() {
        let lease = CliLease {
            lease_id: "lease-1".to_string(),
            identity_device_id: Some("digital-esp32s3".to_string()),
            heartbeat_interval_ms: 1000,
        };
        let resolved = ResolvedUsbHardware {
            hardware_id: "usb-digital-1".to_string(),
            device: "digital-1".to_string(),
            devd: "http://devd".to_string(),
            port_path: Some("mock://esp32s3".to_string()),
            expected_identity_device_id: None,
        };

        let err = validate_cli_lease_identity(&lease, &resolved).unwrap_err();
        assert!(
            err.to_string()
                .contains("not a stable LoadLynx hardware id")
        );
    }

    #[test]
    fn saved_usb_lease_requires_reported_expected_identity() {
        let lease = CliLease {
            lease_id: "lease-1".to_string(),
            identity_device_id: None,
            heartbeat_interval_ms: 1000,
        };
        let resolved = ResolvedUsbHardware {
            hardware_id: "loadlynx-a1b2c3".to_string(),
            device: "digital-1".to_string(),
            devd: "http://devd".to_string(),
            port_path: Some("mock://esp32s3".to_string()),
            expected_identity_device_id: Some("loadlynx-a1b2c3".to_string()),
        };

        let err = validate_cli_lease_identity(&lease, &resolved).unwrap_err();
        assert!(
            err.to_string()
                .contains("expected identity device_id loadlynx-a1b2c3")
        );
        assert!(err.to_string().contains("<missing>"));
    }

    #[tokio::test]
    async fn artifact_selection_scans_saved_port_after_stale_device_id() {
        let state = TestHttpState::default();
        let devd = spawn_artifact_scan_required_test_ipc(state.clone()).await;
        let resolved = ResolvedUsbHardware {
            hardware_id: "loadlynx-a1b2c3".to_string(),
            device: "digital-stale".to_string(),
            devd,
            port_path: Some("mock://esp32s3".to_string()),
            expected_identity_device_id: Some("loadlynx-a1b2c3".to_string()),
        };

        let value = select_device_artifact(
            &Client::new(),
            &resolved,
            Some("manifest.json".to_string()),
            Some("digital-fw".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(
            value.get("device_id").and_then(Value::as_str),
            Some("digital-current")
        );
        assert_eq!(state.scans.load(Ordering::SeqCst), 1);
        let artifact_devices = state
            .artifact_devices
            .lock()
            .expect("artifact devices lock");
        assert_eq!(
            artifact_devices
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["digital-stale", "digital-current"]
        );
    }

    #[test]
    fn hardware_use_updates_last_transport() {
        let mut hardware = vec![
            SavedHardware {
                id: "loadlynx-000010".to_string(),
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
                id: "loadlynx-000030".to_string(),
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
            vec!["loadlynx-000010", "loadlynx-000030"]
        );
    }

    #[test]
    fn available_hardware_payload_keeps_usb_and_http_fallback_separate() {
        let mut registry = HardwareRegistry::default();
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-d4e5f6".to_string(),
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
            "loadlynx-a1b2c3".to_string(),
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
            Some("loadlynx-a1b2c3")
        );
        assert_eq!(
            payload
                .pointer("/http_fallback/0/id")
                .and_then(Value::as_str),
            Some("loadlynx-d4e5f6")
        );
        assert_eq!(payload.get("scan").unwrap(), &Value::Null);
    }

    #[test]
    fn selectors_reject_ambiguous_saved_hardware_inputs() {
        let status_err = ensure_one_status_selector(
            Some(&"http://loadlynx.local".to_string()),
            Some(&"loadlynx-a1b2c3".to_string()),
        )
        .unwrap_err();
        assert!(status_err.to_string().contains("status accepts only one"));

        let usb_err = resolve_usb_target(
            Some("digital-1".to_string()),
            "http://127.0.0.1:30180",
            false,
        )
        .unwrap_err();
        assert!(
            usb_err
                .to_string()
                .contains("saved device not found: digital-1")
        );

        let mode_err =
            validate_mode_first_targets(ModeFirstCommand::Cp, 0, None, Some(1_000), 10_000, 500)
                .unwrap_err();
        assert!(mode_err.to_string().contains("target_p_mw"));
    }

    #[test]
    fn freeze_api_selector_resolves_interactive_saved_device_once() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let previous_home = env::var_os("LOADLYNX_HOME");
        let previous_cwd = env::current_dir().unwrap();
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();

        // Tests serialize environment mutation through TEST_ENV_LOCK.
        unsafe { env::set_var("LOADLYNX_HOME", temp.path()) };
        env::set_current_dir(&project).unwrap();
        write_hardware_registry(
            &temp.path().join("devices.json"),
            &HardwareRegistry {
                hardware: vec![SavedHardware {
                    id: "loadlynx-a1b2c3".to_string(),
                    name: Some("Bench".to_string()),
                    identity: None,
                    last_transport: Some(SavedTransport::Usb),
                    transports: SavedTransports {
                        usb: Some(SavedUsbTransport {
                            device: "digital-1".to_string(),
                            port_path: Some("mock://esp32s3".to_string()),
                            devd: None,
                        }),
                        http: None,
                    },
                    last_seen_unix_seconds: None,
                }],
                ..HardwareRegistry::default()
            },
        )
        .unwrap();

        let frozen = freeze_api_selector(
            ApiSelector {
                url: None,
                device: None,
            },
            "http://127.0.0.1:30180",
            true,
        )
        .unwrap();

        assert_eq!(frozen.url, None);
        assert_eq!(frozen.device.as_deref(), Some("loadlynx-a1b2c3"));

        env::set_current_dir(previous_cwd).unwrap();
        match previous_home {
            Some(value) => unsafe { env::set_var("LOADLYNX_HOME", value) },
            None => unsafe { env::remove_var("LOADLYNX_HOME") },
        }
    }
}
