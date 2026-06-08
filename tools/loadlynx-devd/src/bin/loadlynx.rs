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
    ResolvedHardware, ResolvedUsbHardware, SavedTransport, handle_hardware_command,
    is_stable_hardware_id, mark_default_transport_used, mark_hardware_transport_used,
    resolve_saved_hardware, resolve_usb_target,
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
    post_usb_operation_with_optional_lease, release_cli_lease, request_api_value,
    request_devd_usb_value, request_http_value, resolve_output_enable,
    resolve_scanned_usb_device_for_saved_hardware, run_monitor, saved_usb_device_needs_relookup,
    spawn_cli_lease_heartbeat,
};

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
    Cc {
        target_i_ma: u32,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
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
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
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
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        hardware: Option<String>,
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
                client
                    .get(api_url(&url, "/api/v1/status")?)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?
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
                    ResolvedHardware::Http { hardware_id, url } => {
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
            let resolved = ResolvedUsbHardware {
                expected_identity_device_id: expected_identity_device_id.clone().or(
                    resolved.expected_identity_device_id,
                ),
                ..resolved
            };
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
        Command::Cc {
            target_i_ma,
            url,
            hardware,
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
                hardware,
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
            hardware,
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
                hardware,
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
            hardware,
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
                hardware,
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
        Command::Cc { url, hardware, .. }
        | Command::Cv { url, hardware, .. }
        | Command::Cp { url, hardware, .. } => {
            selector_devd_endpoint(url.as_ref(), None, hardware.as_ref(), default_devd)
                .into_iter()
                .collect()
        }
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
        extract::{Path, State},
        http::StatusCode,
        routing::{get, post},
    };
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
                .push(payload.clone());
            axum::Json(json!({
                "lease_id": "lease-1",
                "identity_device_id": payload.get("expected_identity_device_id").cloned().unwrap_or(Value::Null),
                "heartbeat_interval_ms": 1000
            }))
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
                .push(payload.clone());
            if state.scans.load(Ordering::SeqCst) == 0 {
                (
                    StatusCode::NOT_FOUND,
                    axum::Json(json!({"code": "device_not_found"})),
                )
            } else {
                (
                    StatusCode::OK,
                    axum::Json(json!({
                        "lease_id": "lease-1",
                        "identity_device_id": payload.get("expected_identity_device_id").cloned().unwrap_or(Value::Null),
                        "heartbeat_interval_ms": 1000
                    })),
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

    async fn spawn_artifact_scan_required_test_http(state: TestHttpState) -> String {
        async fn select_artifact_after_scan(
            State(state): State<TestHttpState>,
            Path(device): Path<String>,
            axum::Json(payload): axum::Json<Value>,
        ) -> (StatusCode, axum::Json<Value>) {
            state
                .artifact_devices
                .lock()
                .expect("artifact devices lock")
                .push(device.clone());
            if state.scans.load(Ordering::SeqCst) == 0 {
                (
                    StatusCode::NOT_FOUND,
                    axum::Json(json!({"code": "device_not_found"})),
                )
            } else {
                (
                    StatusCode::OK,
                    axum::Json(json!({"ok": true, "device_id": device, "payload": payload})),
                )
            }
        }

        async fn scan(State(state): State<TestHttpState>) -> axum::Json<Value> {
            state.scans.fetch_add(1, Ordering::SeqCst);
            axum::Json(
                json!({"devices": [{"id": "digital-current", "digital_target": {"port_path": "mock://esp32s3"}}]}),
            )
        }

        let app = Router::new()
            .route(
                "/api/v1/devices/{device}/artifact",
                post(select_artifact_after_scan),
            )
            .route("/api/v1/devices/scan", post(scan))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn spawn_identity_mismatch_then_scan_test_http(state: TestHttpState) -> String {
        async fn create_lease_after_scan(
            State(state): State<TestHttpState>,
            axum::Json(payload): axum::Json<Value>,
        ) -> (StatusCode, axum::Json<Value>) {
            state.lease_creates.fetch_add(1, Ordering::SeqCst);
            state
                .lease_payloads
                .lock()
                .expect("lease payloads lock")
                .push(payload.clone());
            if state.scans.load(Ordering::SeqCst) == 0 {
                (
                    StatusCode::CONFLICT,
                    axum::Json(json!({"code": "identity_confirmation_mismatch"})),
                )
            } else {
                (
                    StatusCode::OK,
                    axum::Json(json!({
                        "lease_id": "lease-1",
                        "identity_device_id": payload.get("expected_identity_device_id").cloned().unwrap_or(Value::Null),
                        "heartbeat_interval_ms": 1000
                    })),
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
    async fn real_usb_firmware_operation_creates_cli_lease() {
        let state = TestHttpState::default();
        let devd = spawn_test_http(state.clone()).await;
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
            "cc",
            "2000",
            "--hardware",
            "usb-digital-1",
            "--preset-id",
            "2",
            "--disable",
        ])
        .unwrap();
        match cli.command {
            Command::Cc {
                target_i_ma,
                hardware,
                preset_id,
                disable,
                ..
            } => {
                assert_eq!(target_i_ma, 2000);
                assert_eq!(hardware.as_deref(), Some("usb-digital-1"));
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

        let cli = Cli::try_parse_from(["loadlynx", "cv", "24500", "--hardware", "usb-digital-1"])
            .unwrap();
        match cli.command {
            Command::Cv {
                target_v_mv,
                hardware,
                ..
            } => {
                assert_eq!(target_v_mv, 24_500);
                assert_eq!(hardware.as_deref(), Some("usb-digital-1"));
            }
            _ => panic!("expected cv command"),
        }

        let cli = Cli::try_parse_from(["loadlynx", "cp", "60000", "--hardware", "usb-digital-1"])
            .unwrap();
        match cli.command {
            Command::Cp {
                target_p_mw,
                hardware,
                ..
            } => {
                assert_eq!(target_p_mw, 60_000);
                assert_eq!(hardware.as_deref(), Some("usb-digital-1"));
            }
            _ => panic!("expected cp command"),
        }

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
                hardware: None,
            },
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
                hardware: None,
            },
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
        let devd = spawn_identity_mismatch_then_scan_test_http(state.clone()).await;
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
        let devd = spawn_artifact_scan_required_test_http(state.clone()).await;
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

        let mode_err =
            validate_mode_first_targets(ModeFirstCommand::Cp, 0, None, Some(1_000), 10_000, 500)
                .unwrap_err();
        assert!(mode_err.to_string().contains("target_p_mw"));
    }
}
