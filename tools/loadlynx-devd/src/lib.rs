use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response, sse::Event, sse::Sse},
    routing::{delete, get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    env, fs,
    future::Future,
    io,
    net::SocketAddr,
    path::PathBuf,
    process::Stdio,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex, mpsc as std_mpsc},
    thread,
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    process::Command,
    sync::{Notify, broadcast, oneshot},
    time::sleep,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
};

mod compat_response;
mod serial_response;

use compat_response::{
    expand_compact_calibration_profile, identity_data_from_serial_response,
    merge_presets_from_data, pd_post_response_data, pd_response_data, presets_data_from_map,
    serial_response_data, serial_response_data_required, status_data_from_serial_response,
};
use serial_response::{
    ExtractedSerialFrame, SerialProtocolFrame, SerialProtocolProbe, extract_serial_json_frames,
    infer_serial_response_from_fragments, infer_serial_response_from_text, sanitize_trace_text,
    serial_probe_has_mismatched_response, serial_request_id_matches_op,
    serial_response_for_request,
};

pub const DEFAULT_BIND: &str = "127.0.0.1:30180";
pub const DEFAULT_DEVD_URL: &str = "http://127.0.0.1:30180";
pub const DEFAULT_IPC_IDLE_TIMEOUT_SECS: u64 = 30;
pub const FLASH_CONFIRMATION_TEXT: &str = "yes";
pub const DEFAULT_DIGITAL_USB_PORT_FILE: &str = ".esp32-port";
pub const DEFAULT_ANALOG_PROBE_FILE: &str = ".stm32-port";
const DEFAULT_DIGITAL_USB_PORT_SELECTOR_SOURCE: &str = ".esp32-port";
const DEFAULT_ANALOG_PROBE_SELECTOR_SOURCE: &str = ".stm32-port";
const ESPFLASH_ENV: &str = "LOADLYNX_ESPFLASH";
const DEFAULT_ESPFLASH: &str = "espflash";
const PROBE_RS_ENV: &str = "LOADLYNX_PROBE_RS";
const DEFAULT_PROBE_RS: &str = "probe-rs";
const ANALOG_PROBE_CHIP: &str = "STM32G431CB";
const ANALOG_PROBE_PROTOCOL: &str = "swd";
const ANALOG_PROBE_SPEED_KHZ: u32 = 4_000;
pub const WEB_LEASE_HEARTBEAT_INTERVAL_MS: u64 = 2_000;
pub const WEB_LEASE_TTL_MS: u64 = 8_000;
const EVENT_LIMIT: usize = 1_000;
const LOG_LIMIT: usize = 500;
const TRACE_LIMIT: usize = 2_000;
const SERIAL_PROBE_BAUD: u32 = 115_200;
// Keep serial reads short so recoverable USB JSONL responses do not inherit
// a fixed half-second wait on every warmup/read slice.
const SERIAL_PROBE_TIMEOUT_MS: u64 = 25;
const SERIAL_PROTOCOL_TIMEOUT_MS: u64 = 5_000;
const SERIAL_WIFI_WAIT_PROTOCOL_TIMEOUT_MS: u64 = 35_000;
const SERIAL_STATUS_POLL_INTERVAL_MS: u64 = 200;
const SERIAL_STATUS_REQUEST_TIMEOUT_MS: u64 = 750;
const SERIAL_STATUS_CACHE_MAX_AGE_MS: i64 = 1_500;
const SERIAL_PROBE_MAX_BYTES: usize = 32768;
const SERIAL_COMMAND_QUEUE_LIMIT: usize = 16;
const SERIAL_OPERATION_WAIT_MS: u64 = 10_000;
const SERIAL_WIFI_WAIT_OPERATION_WAIT_MS: u64 = 40_000;
const STATUS_CACHE_MAX_AGE_MS: i64 = 500;
const LOADLYNX_PRESET_COUNT: usize = 5;
pub const HOST_TOOLS_VERSION: &str = match option_env!("LOADLYNX_RELEASE_VERSION") {
    Some(version) => version,
    None => match option_env!("LOADLYNX_PROJECT_VERSION") {
        Some(version) => version,
        None => env!("CARGO_PKG_VERSION"),
    },
};

pub fn default_ipc_endpoint() -> String {
    #[cfg(windows)]
    {
        r"\\.\pipe\loadlynx-devd".to_string()
    }
    #[cfg(not(windows))]
    {
        let base = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir);
        base.join("loadlynx-devd.sock")
            .to_string_lossy()
            .into_owned()
    }
}

#[derive(Debug, Clone)]
pub struct DevdConfig {
    pub bind: SocketAddr,
    pub web_root: Option<PathBuf>,
    pub allow_dev_cors: bool,
    pub repo_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub endpoint: String,
    pub idle_timeout: Duration,
    pub repo_root: PathBuf,
}

impl IpcConfig {
    pub fn new(endpoint: String, idle_timeout: Duration) -> Self {
        Self {
            endpoint,
            idle_timeout,
            repo_root: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub op: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl DevdConfig {
    pub fn new(bind: SocketAddr, web_root: Option<PathBuf>, allow_dev_cors: bool) -> Self {
        Self {
            bind,
            web_root,
            allow_dev_cors,
            repo_root: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<DevdState>>,
    serial: Arc<Mutex<SerialOwnerRegistry>>,
    events: broadcast::Sender<DevdEvent>,
    repo_root: PathBuf,
    #[cfg(test)]
    mock_serial_responses: Arc<Mutex<VecDeque<SerialProtocolProbe>>>,
}

#[derive(Debug, Default)]
struct DevdState {
    devices: HashMap<String, DeviceRecord>,
    artifacts: HashMap<String, FirmwareArtifact>,
    leases: HashMap<String, WebLease>,
    events: VecDeque<DevdEvent>,
}

#[derive(Default)]
struct SerialOwnerRegistry {
    owners: HashMap<String, SerialOwnerHandle>,
    exclusive_ports: HashMap<String, String>,
    next_owner_id: u64,
}

struct SerialOwnerHandle {
    id: u64,
    tx: std_mpsc::SyncSender<SerialWorkerCommand>,
    join: thread::JoinHandle<()>,
}

struct SerialWorkerCommand {
    request: SerialJsonlCommand,
    reply: oneshot::Sender<Result<SerialProtocolProbe, SerialWorkerError>>,
}

#[derive(Clone)]
struct SerialJsonlCommand {
    device_id: String,
    port_path: String,
    request_id: String,
    op: String,
    extra: Option<Value>,
}

#[derive(Debug)]
struct SerialWorkerError {
    code: &'static str,
    message: String,
    retryable: bool,
}

struct SerialExclusiveGuard {
    state: AppState,
    port_path: String,
}

impl Drop for SerialExclusiveGuard {
    fn drop(&mut self) {
        clear_serial_exclusive_and_resume_owner(&self.state, &self.port_path);
    }
}

#[derive(Debug, Clone)]
struct WebLease {
    lease_id: String,
    device_id: String,
    identity_device_id: Option<String>,
    bind_probe: bool,
    legacy_preflash_only: bool,
    port_path: Option<String>,
    expires_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    DigitalEsp32s3,
    AnalogStm32g431,
    LanHttp,
    Mock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetCandidate {
    pub kind: TargetKind,
    pub display_name: String,
    pub port_path: Option<String>,
    pub probe_selector: Option<String>,
    pub lan_base_url: Option<String>,
    pub selector_source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitalUsbPortCandidate {
    pub port_path: String,
    pub display_name: String,
    pub recognized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub id: String,
    pub display_name: String,
    pub connection: ConnectionState,
    pub digital_target: Option<TargetCandidate>,
    pub analog_target: Option<TargetCandidate>,
    pub lan_endpoint: Option<String>,
    pub identity: Option<Value>,
    pub usb_pd_cache: Option<Value>,
    pub status_cache: Option<Value>,
    pub control_cache: Option<Value>,
    pub status_meta_cache: Option<Value>,
    pub status_cache_updated_at_ms: Option<i64>,
    pub usb_status_cache: Option<Value>,
    pub usb_status_generation: u64,
    pub usb_status_sampled_at_ms: Option<i64>,
    pub usb_status_source: Option<String>,
    pub selected_artifact_id: Option<String>,
    pub log_decode: LogDecodeState,
    pub logs: VecDeque<SessionLog>,
    pub trace: VecDeque<SessionTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connected,
    Busy,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDecodeState {
    pub status: String,
    pub reason: Option<String>,
    pub artifact_id: Option<String>,
}

impl Default for LogDecodeState {
    fn default() -> Self {
        Self {
            status: "unverified".to_string(),
            reason: Some("no artifact selected".to_string()),
            artifact_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareCatalog {
    pub schema_version: String,
    pub artifacts: Vec<FirmwareArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FirmwareManifest {
    Catalog(FirmwareCatalog),
    Artifact(Box<FirmwareArtifact>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareArtifact {
    pub artifact_id: String,
    pub name: String,
    pub target: TargetKind,
    pub package_version: String,
    pub git_sha: String,
    pub build_id: String,
    pub build_profile: String,
    pub features: Vec<String>,
    pub protocol: String,
    pub defmt: DefmtMetadata,
    pub files: Vec<ArtifactFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefmtMetadata {
    pub enabled: bool,
    pub encoding: String,
    pub elf_sha256: Option<String>,
    pub table_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFile {
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub flash_address: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevdEvent {
    pub id: String,
    pub timestamp: String,
    pub device_id: Option<String>,
    pub kind: String,
    pub message: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTrace {
    pub id: String,
    pub timestamp: String,
    pub direction: String,
    pub summary: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorEnvelope {
    pub error: ApiError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}

impl From<HttpError> for ApiError {
    fn from(error: HttpError) -> Self {
        error.0
    }
}

#[derive(Debug)]
pub struct HttpError(ApiError, StatusCode);

#[derive(Debug, Deserialize)]
struct BindRequest {
    alias: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectRequest {
    identity: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ArtifactSelectRequest {
    manifest_path: Option<String>,
    artifact_id: Option<String>,
    artifact: Option<FirmwareArtifact>,
}

#[derive(Debug, Deserialize)]
struct FlashRequest {
    target: Option<TargetKind>,
    artifact_id: Option<String>,
    dry_run: Option<bool>,
    lease_id: Option<String>,
    confirmation_phrase: Option<String>,
    expected_identity_device_id: Option<String>,
    acknowledge_non_project_firmware: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ResetRequest {
    target: Option<TargetKind>,
    dry_run: Option<bool>,
    lease_id: Option<String>,
    confirmation_phrase: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LeaseRequest {
    device_id: String,
    expected_identity_device_id: Option<String>,
    bind_probe: Option<bool>,
    allow_legacy_preflash_identity_fallback: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    device_id: Option<String>,
    lease_id: Option<String>,
    logs_limit: Option<usize>,
    trace_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CompatQuery {
    device_id: Option<String>,
    lease_id: Option<String>,
    #[serde(default)]
    fresh: bool,
    #[serde(default)]
    cache: bool,
}

#[derive(Debug, Deserialize)]
struct CcRequest {
    enable: bool,
    target_i_ma: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PdRequest {
    mode: Option<String>,
    object_pos: Option<u8>,
    target_mv: Option<u32>,
    i_req_ma: Option<u32>,
    allow_extended_voltage: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WifiSetRequest {
    ssid: String,
    psk: String,
    #[serde(default)]
    wait: bool,
}

pub async fn serve(config: DevdConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let state = AppState::new(config.repo_root.clone());
    let router = router(state, config.web_root, config.allow_dev_cors);
    ensure_loopback_bind(&config.bind)?;
    let listener = TcpListener::bind(config.bind).await?;
    tracing::info!(
        "loadlynx-devd bridge-http listening on http://{}",
        config.bind
    );
    axum::serve(listener, router).await?;
    Ok(())
}

pub async fn serve_ipc(config: IpcConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let state = AppState::new(config.repo_root.clone());

    tracing::info!("loadlynx-devd IPC listening on {}", config.endpoint);

    #[cfg(windows)]
    {
        return serve_ipc_windows(config, state).await;
    }
    #[cfg(not(windows))]
    {
        serve_ipc_unix(config, state).await
    }
}

pub async fn ipc_request(
    endpoint: &str,
    request: IpcRequest,
) -> Result<IpcResponse, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        return ipc_request_windows(endpoint, request).await;
    }
    #[cfg(not(windows))]
    {
        return ipc_request_unix(endpoint, request).await;
    }
}

#[cfg(not(windows))]
async fn serve_ipc_unix(
    config: IpcConfig,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = PathBuf::from(&config.endpoint);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    remove_stale_unix_socket(&path).await?;
    let listener = tokio::net::UnixListener::bind(&path)?;
    let active_connections = Arc::new(AtomicUsize::new(0));
    let idle_notify = Arc::new(Notify::new());
    loop {
        if active_connections.load(Ordering::SeqCst) == 0 {
            if config.idle_timeout.is_zero() {
                let (stream, _peer) = listener.accept().await?;
                let state = state.clone();
                spawn_tracked_ipc_connection(
                    active_connections.clone(),
                    idle_notify.clone(),
                    async move { handle_ipc_connection_unix(stream, state).await },
                );
            } else {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (stream, _peer) = accepted?;
                        let state = state.clone();
                        spawn_tracked_ipc_connection(active_connections.clone(), idle_notify.clone(), async move {
                            handle_ipc_connection_unix(stream, state).await
                        });
                    }
                    _ = sleep(config.idle_timeout) => {
                        tracing::info!(
                            "loadlynx-devd IPC idle timeout after {}s; exiting",
                            config.idle_timeout.as_secs()
                        );
                        let _ = fs::remove_file(&path);
                        return Ok(());
                    }
                }
            }
        } else {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, _peer) = accepted?;
                    let state = state.clone();
                    spawn_tracked_ipc_connection(active_connections.clone(), idle_notify.clone(), async move {
                        handle_ipc_connection_unix(stream, state).await
                    });
                }
                _ = idle_notify.notified() => {}
            }
        }
    }
}

#[cfg(not(windows))]
async fn remove_stale_unix_socket(path: &PathBuf) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    match tokio::net::UnixStream::connect(path).await {
        Ok(_stream) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("IPC endpoint is already in use: {}", path.to_string_lossy()),
        )),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(path)
        }
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
async fn serve_ipc_windows(
    config: IpcConfig,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pipe_name = normalize_named_pipe_name(&config.endpoint);
    let active_connections = Arc::new(AtomicUsize::new(0));
    let idle_notify = Arc::new(Notify::new());
    loop {
        let server = tokio::net::windows::named_pipe::ServerOptions::new().create(&pipe_name)?;
        if active_connections.load(Ordering::SeqCst) == 0 {
            if config.idle_timeout.is_zero() {
                server.connect().await?;
                let state = state.clone();
                spawn_tracked_ipc_connection(
                    active_connections.clone(),
                    idle_notify.clone(),
                    async move { handle_ipc_connection_windows(server, state).await },
                );
            } else {
                tokio::select! {
                    connected = server.connect() => {
                        connected?;
                        let state = state.clone();
                        spawn_tracked_ipc_connection(active_connections.clone(), idle_notify.clone(), async move {
                            handle_ipc_connection_windows(server, state).await
                        });
                    }
                    _ = sleep(config.idle_timeout) => {
                        tracing::info!(
                            "loadlynx-devd IPC idle timeout after {}s; exiting",
                            config.idle_timeout.as_secs()
                        );
                        return Ok(());
                    }
                }
            }
        } else {
            tokio::select! {
                connected = server.connect() => {
                    connected?;
                    let state = state.clone();
                    spawn_tracked_ipc_connection(active_connections.clone(), idle_notify.clone(), async move {
                        handle_ipc_connection_windows(server, state).await
                    });
                }
                _ = idle_notify.notified() => {}
            }
        }
    }
}

fn spawn_tracked_ipc_connection<F>(
    active_connections: Arc<AtomicUsize>,
    idle_notify: Arc<Notify>,
    handler: F,
) where
    F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
    active_connections.fetch_add(1, Ordering::SeqCst);
    tokio::spawn(async move {
        if let Err(error) = handler.await {
            tracing::warn!("IPC connection failed: {error}");
        }
        active_connections.fetch_sub(1, Ordering::SeqCst);
        idle_notify.notify_waiters();
    });
}

#[cfg(not(windows))]
async fn ipc_request_unix(
    endpoint: &str,
    request: IpcRequest,
) -> Result<IpcResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = tokio::net::UnixStream::connect(endpoint).await?;
    ipc_roundtrip(&mut stream, request).await
}

#[cfg(windows)]
async fn ipc_request_windows(
    endpoint: &str,
    request: IpcRequest,
) -> Result<IpcResponse, Box<dyn std::error::Error + Send + Sync>> {
    let pipe_name = normalize_named_pipe_name(endpoint);
    let mut stream = loop {
        match tokio::net::windows::named_pipe::ClientOptions::new().open(&pipe_name) {
            Ok(stream) => break stream,
            Err(error) => {
                if let Some(code) = error.raw_os_error() {
                    if code == 231 {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                }
                return Err(Box::new(error));
            }
        }
    };
    ipc_roundtrip(&mut stream, request).await
}

async fn ipc_roundtrip<S>(
    stream: &mut S,
    request: IpcRequest,
) -> Result<IpcResponse, Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let line = serde_json::to_vec(&request)?;
    stream.write_all(&line).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let response: IpcResponse = serde_json::from_str(response.trim_end())?;
    Ok(response)
}

#[cfg(not(windows))]
async fn handle_ipc_connection_unix(
    stream: tokio::net::UnixStream,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    handle_ipc_connection(stream, state).await
}

#[cfg(windows)]
async fn handle_ipc_connection_windows(
    stream: tokio::net::windows::named_pipe::NamedPipeServer,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    handle_ipc_connection(stream, state).await
}

async fn handle_ipc_connection<S>(
    stream: S,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<IpcRequest>(&line) {
            Ok(request) => dispatch_ipc_operation(&state, request).await,
            Err(error) => IpcResponse {
                ok: false,
                result: None,
                error: Some(ApiError {
                    code: "ipc_invalid_json".to_string(),
                    message: error.to_string(),
                    retryable: false,
                    details: None,
                }),
            },
        };
        writer
            .write_all(serde_json::to_string(&response)?.as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    Ok(())
}

async fn dispatch_ipc_operation(state: &AppState, request: IpcRequest) -> IpcResponse {
    match dispatch_ipc_operation_result(state.clone(), request).await {
        Ok(value) => IpcResponse {
            ok: true,
            result: Some(value),
            error: None,
        },
        Err(error) => IpcResponse {
            ok: false,
            result: None,
            error: Some(error.into()),
        },
    }
}

async fn dispatch_ipc_operation_result(
    state: AppState,
    request: IpcRequest,
) -> Result<Value, HttpError> {
    let params = request.params;
    match request.op.as_str() {
        "health" => Ok(health().await.0),
        "devices.list" => Ok(list_devices(State(state)).await.0),
        "devices.scan" => Ok(scan_devices(State(state)).await?.0),
        "devices.artifact.select" => {
            let id = required_string(&params, "device_id")?;
            let input: ArtifactSelectRequest = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(select_artifact(State(state), Path(id), Json(input))
                .await?
                .0)
        }
        "devices.flash" => {
            let id = required_string(&params, "device_id")?;
            let input: FlashRequest = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(flash_device(State(state), Path(id), Json(input)).await?.0)
        }
        "devices.reset" => {
            let id = required_string(&params, "device_id")?;
            let input: ResetRequest = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(reset_device(State(state), Path(id), Some(Json(input)))
                .await?
                .0)
        }
        "devices.session" => {
            let id = required_string(&params, "device_id")?;
            let query: SessionQuery = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(device_session(State(state), Path(id), Query(query))
                .await?
                .0)
        }
        "serial.lease.create" => {
            let input: LeaseRequest = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(create_lease(State(state), Json(input)).await?.0)
        }
        "serial.lease.heartbeat" => {
            let lease_id = required_string(&params, "lease_id")?;
            Ok(heartbeat_lease(State(state), Path(lease_id)).await?.0)
        }
        "serial.lease.release" => {
            let lease_id = required_string(&params, "lease_id")?;
            Ok(release_lease(State(state), Path(lease_id)).await?.0)
        }
        "compat.identity" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_identity(State(state), Query(query)).await?.0)
        }
        "compat.status" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_status(State(state), Query(query)).await?.0)
        }
        "compat.network" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_network(State(state), Query(query)).await?.0)
        }
        "compat.session" => {
            let query: SessionQuery = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(compat_session(State(state), Query(query)).await?.0)
        }
        "compat.pd.get" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_pd_get(State(state), Query(query)).await?.0)
        }
        "compat.pd.post" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(compat_pd_post(State(state), Query(query), body.to_string())
                .await?
                .0)
        }
        "compat.cc" => {
            let (query, body) = compat_query_and_body(params)?;
            let input: CcRequest = serde_json::from_value(body)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(compat_cc(State(state), Query(query), Json(input)).await?.0)
        }
        "compat.wifi.get" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_wifi_get(State(state), Query(query)).await?.0)
        }
        "compat.wifi.credentials" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_wifi_credentials_get(State(state), Query(query))
                .await?
                .0)
        }
        "compat.wifi.post" => {
            let (query, body) = compat_query_and_body(params)?;
            let input: WifiSetRequest = serde_json::from_value(body)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(compat_wifi_post(State(state), Query(query), Json(input))
                .await?
                .0)
        }
        "compat.wifi.delete" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_wifi_delete(State(state), Query(query)).await?.0)
        }
        "compat.control.get" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_control_get(State(state), Query(query)).await?.0)
        }
        "compat.control.post" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_control_post(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.presets.get" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_presets_get(State(state), Query(query)).await?.0)
        }
        "compat.presets.post" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_presets_post(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.presets.apply" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_presets_apply(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.calibration.profile" => {
            let query = compat_query_from_params(params)?;
            Ok(compat_calibration_profile(State(state), Query(query))
                .await?
                .0)
        }
        "compat.calibration.apply" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_calibration_apply(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.calibration.commit" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_calibration_commit(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.calibration.reset" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_calibration_reset(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.calibration.mode" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_calibration_mode(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.soft_reset" => {
            let (query, body) = compat_query_and_body(params)?;
            Ok(
                compat_soft_reset(State(state), Query(query), body.to_string())
                    .await?
                    .0,
            )
        }
        "compat.diagnostics.export" => {
            let query: SessionQuery = serde_json::from_value(params)
                .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))?;
            Ok(compat_diagnostics_export(State(state), Query(query))
                .await?
                .0)
        }
        _ => Err(HttpError::bad_request(
            "ipc_unknown_operation",
            format!("unknown IPC operation `{}`", request.op),
        )),
    }
}

fn required_string(params: &Value, field: &str) -> Result<String, HttpError> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| HttpError::bad_request("ipc_invalid_params", format!("missing `{field}`")))
}

fn compat_query_from_params(params: Value) -> Result<CompatQuery, HttpError> {
    serde_json::from_value(params)
        .map_err(|error| HttpError::bad_request("ipc_invalid_params", error.to_string()))
}

fn compat_query_and_body(params: Value) -> Result<(CompatQuery, Value), HttpError> {
    let query = CompatQuery {
        device_id: params
            .get("device_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        lease_id: params
            .get("lease_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        fresh: params
            .get("fresh")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        cache: params
            .get("cache")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    let body = params.get("body").cloned().unwrap_or(Value::Null);
    Ok((query, body))
}

#[cfg(windows)]
fn normalize_named_pipe_name(raw: &str) -> String {
    if raw.starts_with(r"\\.\pipe\") {
        raw.to_string()
    } else {
        format!(r"\\.\pipe\{}", raw)
    }
}

pub fn ensure_loopback_bind(
    bind: &SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if bind.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!("loadlynx-devd may only bind loopback addresses; got {bind}").into())
    }
}

impl AppState {
    pub fn new(repo_root: PathBuf) -> Self {
        let (events, _) = broadcast::channel(EVENT_LIMIT);
        let state = Self {
            inner: Arc::new(Mutex::new(DevdState::default())),
            serial: Arc::new(Mutex::new(SerialOwnerRegistry::default())),
            events,
            repo_root,
            #[cfg(test)]
            mock_serial_responses: Arc::new(Mutex::new(VecDeque::new())),
        };
        seed_mock_device(&state);
        spawn_lease_reaper(state.clone());
        state
    }
}

fn router(state: AppState, web_root: Option<PathBuf>, allow_dev_cors: bool) -> Router {
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/api/v1/ping", get(health))
        .route("/api/v1/devices", get(list_devices))
        .route("/api/v1/devices/scan", post(scan_devices))
        .route("/api/v1/devices/{id}/bind", post(bind_device))
        .route("/api/v1/devices/{id}/connect", post(connect_device))
        .route("/api/v1/devices/{id}/disconnect", post(disconnect_device))
        .route("/api/v1/devices/{id}/binding", delete(unbind_device))
        .route("/api/v1/devices/{id}/identity", get(device_identity))
        .route("/api/v1/devices/{id}/status", get(device_status))
        .route("/api/v1/devices/{id}/network", get(device_network))
        .route("/api/v1/devices/{id}/session", get(device_session))
        .route("/api/v1/devices/{id}/events", get(device_events))
        .route(
            "/api/v1/devices/{id}/artifact",
            get(device_artifact).post(select_artifact),
        )
        .route("/api/v1/devices/{id}/flash", post(flash_device))
        .route("/api/v1/devices/{id}/reset", post(reset_device))
        .route("/api/v1/devices/{id}/monitor/start", post(monitor_start))
        .route("/api/v1/devices/{id}/monitor/stop", post(monitor_stop))
        .route("/api/v1/serial/lease", post(create_lease))
        .route(
            "/api/v1/serial/lease/{lease_id}",
            post(heartbeat_lease).delete(release_lease),
        )
        .route("/api/v1/serial/session", get(compat_session))
        .route("/api/v1/serial/events", get(compat_events))
        .route("/api/v1/identity", get(compat_identity))
        .route("/api/v1/status", get(compat_status))
        .route("/api/v1/network", get(compat_network))
        .route(
            "/api/v1/wifi",
            get(compat_wifi_get)
                .post(compat_wifi_post)
                .delete(compat_wifi_delete),
        )
        .route("/api/v1/wifi/credentials", get(compat_wifi_credentials_get))
        .route("/api/v1/cc", post(compat_cc))
        .route("/api/v1/pd", get(compat_pd_get).post(compat_pd_post))
        .route(
            "/api/v1/control",
            get(compat_control_get)
                .post(compat_control_post)
                .put(compat_control_post),
        )
        .route(
            "/api/v1/presets",
            get(compat_presets_get)
                .post(compat_presets_post)
                .put(compat_presets_post),
        )
        .route("/api/v1/presets/apply", post(compat_presets_apply))
        .route(
            "/api/v1/calibration/profile",
            get(compat_calibration_profile),
        )
        .route("/api/v1/calibration/apply", post(compat_calibration_apply))
        .route(
            "/api/v1/calibration/commit",
            post(compat_calibration_commit),
        )
        .route("/api/v1/calibration/reset", post(compat_calibration_reset))
        .route("/api/v1/calibration/mode", post(compat_calibration_mode))
        .route("/api/v1/soft-reset", post(compat_soft_reset))
        .route("/api/v1/diagnostics", get(compat_diagnostics_export))
        .route("/api/v1/diagnostics/export", get(compat_diagnostics_export))
        .with_state(state);

    if allow_dev_cors {
        router = router.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _request_parts| {
                    is_loopback_dev_origin(origin)
                }))
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(tower_http::cors::Any),
        );
    }

    if let Some(web_root) = web_root {
        router = router.fallback_service(ServeDir::new(web_root));
    }
    router
}

fn is_loopback_dev_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };
    ["localhost:", "127.0.0.1:", "[::1]:"]
        .iter()
        .filter_map(|prefix| rest.strip_prefix(prefix))
        .any(valid_port)
}

fn valid_port(port: &str) -> bool {
    port.parse::<u16>().is_ok()
}

async fn health() -> Json<Value> {
    Json(json!({"ok": true, "service": "loadlynx-devd"}))
}

async fn list_devices(State(state): State<AppState>) -> Json<Value> {
    cleanup_expired_leases(&state);
    let guard = state.inner.lock().expect("state lock");
    Json(json!({
        "devices": guard.devices.values().cloned().collect::<Vec<_>>(),
        "leases": guard.leases.values().map(lease_json).collect::<Vec<_>>()
    }))
}

async fn scan_devices(State(state): State<AppState>) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let mut discovered = Vec::new();
    let default_usb_port = read_default_digital_usb_port(&state.repo_root);
    let default_analog_probe = read_default_analog_probe_selector(&state.repo_root);
    let analog_candidate = default_analog_probe
        .as_deref()
        .map(default_analog_probe_candidate);
    discovered.extend(scan_serial_targets(default_usb_port.as_deref()));
    if let Some(candidate) = analog_candidate.clone() {
        discovered.push(candidate);
    }

    let mut guard = state.inner.lock().expect("state lock");
    for candidate in discovered {
        let id = stable_candidate_id(&candidate);
        let entry = guard
            .devices
            .entry(id.clone())
            .or_insert_with(|| DeviceRecord {
                id,
                display_name: candidate.display_name.clone(),
                connection: ConnectionState::Disconnected,
                digital_target: None,
                analog_target: None,
                lan_endpoint: candidate.lan_base_url.clone(),
                identity: None,
                usb_pd_cache: None,
                status_cache: None,
                control_cache: None,
                status_meta_cache: None,
                status_cache_updated_at_ms: None,
                usb_status_cache: None,
                usb_status_generation: 0,
                usb_status_sampled_at_ms: None,
                usb_status_source: None,
                selected_artifact_id: None,
                log_decode: LogDecodeState::default(),
                logs: VecDeque::new(),
                trace: VecDeque::new(),
            });
        match candidate.kind {
            TargetKind::DigitalEsp32s3 => {
                entry.digital_target = Some(candidate);
                if entry.analog_target.is_none()
                    && let Some(analog_candidate) = analog_candidate.clone()
                {
                    entry.analog_target = Some(analog_candidate);
                }
            }
            TargetKind::AnalogStm32g431 => entry.analog_target = Some(candidate),
            TargetKind::LanHttp => entry.lan_endpoint = candidate.lan_base_url.clone(),
            TargetKind::Mock => {}
        }
    }
    let devices = guard.devices.values().cloned().collect::<Vec<_>>();
    drop(guard);
    emit(
        &state,
        None,
        "scan",
        "device scan completed",
        json!({"count": devices.len()}),
    );
    Ok(Json(json!({"devices": devices})))
}

async fn bind_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<BindRequest>,
) -> Result<Json<Value>, HttpError> {
    let mut guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get_mut(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    if let Some(alias) = input.alias {
        device.display_name = alias;
    }
    Ok(Json(json!({"device": device.clone()})))
}

async fn unbind_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let mut guard = state.inner.lock().expect("state lock");
    let removed = guard.devices.remove(&id).is_some();
    Ok(Json(json!({"ok": true, "removed": removed})))
}

async fn connect_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<ConnectRequest>>,
) -> Result<Json<Value>, HttpError> {
    let mut guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get_mut(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    device.connection = ConnectionState::Connected;
    if let Some(Json(input)) = body
        && input.identity.is_some()
    {
        device.identity = input.identity;
    }
    if device.identity.is_none() {
        device.identity = Some(mock_identity(&device.id, &device.display_name));
    }
    push_log(device, "info", "devd", "device connected");
    let device = device.clone();
    drop(guard);
    emit(&state, Some(id), "connect", "device connected", json!({}));
    Ok(Json(json!({"device": device})))
}

async fn disconnect_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    disconnect_device_inner(&state, &id)?;
    Ok(Json(json!({"ok": true})))
}

fn disconnect_device_inner(state: &AppState, id: &str) -> Result<(), HttpError> {
    let mut guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get_mut(id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    device.connection = ConnectionState::Disconnected;
    push_log(device, "info", "devd", "device disconnected");
    drop(guard);
    emit(
        state,
        Some(id.to_string()),
        "disconnect",
        "device disconnected",
        json!({}),
    );
    Ok(())
}

async fn device_identity(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    let identity = device
        .identity
        .clone()
        .unwrap_or_else(|| mock_identity(&device.id, &device.display_name));
    Ok(Json(identity))
}

async fn device_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    let cached = cached_status_bundle(device);
    Ok(Json(json!({
        "device_id": id,
        "connection": device.connection,
        "targets": {
            "digital": device.digital_target,
            "analog": device.analog_target,
            "lan": device.lan_endpoint
        },
        "log_decode": device.log_decode,
        "status_cache": cached.as_ref().and_then(|value| value.get("status")).cloned(),
        "control_cache": cached.as_ref().and_then(|value| value.get("control")).cloned(),
        "status_meta_cache": cached.as_ref().and_then(|value| value.get("status_meta")).cloned(),
        "status_cache_updated_at_ms": device.status_cache_updated_at_ms
    })))
}

async fn device_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    Ok(Json(json!({
        "lan_endpoint": device.lan_endpoint,
        "mdns": device.identity.as_ref().and_then(|i| i.get("hostname")).cloned()
    })))
}

async fn select_artifact(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ArtifactSelectRequest>,
) -> Result<Json<Value>, HttpError> {
    let requested_artifact_id = input.artifact_id;
    let mut loaded_artifact_ids = Vec::new();
    if let Some(artifact) = input.artifact {
        loaded_artifact_ids.push(artifact.artifact_id.clone());
        state
            .inner
            .lock()
            .expect("state lock")
            .artifacts
            .insert(artifact.artifact_id.clone(), artifact);
    } else if let Some(path) = input.manifest_path {
        let artifacts = read_manifest(&path)?;
        let mut guard = state.inner.lock().expect("state lock");
        for artifact in artifacts {
            loaded_artifact_ids.push(artifact.artifact_id.clone());
            guard
                .artifacts
                .insert(artifact.artifact_id.clone(), artifact);
        }
    }
    let artifact_id = match (requested_artifact_id, loaded_artifact_ids.as_slice()) {
        (Some(artifact_id), _) => artifact_id,
        (None, [artifact_id]) => artifact_id.clone(),
        (None, []) => {
            return Err(HttpError::bad_request(
                "artifact_missing",
                "artifact_id, artifact or manifest_path required",
            ));
        }
        (None, _) => {
            return Err(HttpError::bad_request(
                "artifact_id_required",
                "manifest_path loaded multiple artifacts; artifact_id is required",
            ));
        }
    };
    let mut guard = state.inner.lock().expect("state lock");
    let selected = guard
        .artifacts
        .get(&artifact_id)
        .cloned()
        .ok_or_else(|| HttpError::not_found("artifact_not_found", "artifact is not loaded"))?;
    let device = guard
        .devices
        .get_mut(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    device.selected_artifact_id = Some(artifact_id.clone());
    apply_artifact_match(device, Some(&selected));
    let log_decode = device.log_decode.clone();
    Ok(Json(
        json!({"artifact": selected, "log_decode": log_decode}),
    ))
}

async fn device_artifact(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    let artifact = device
        .selected_artifact_id
        .as_ref()
        .and_then(|id| guard.artifacts.get(id));
    Ok(Json(
        json!({"artifact": artifact, "log_decode": device.log_decode}),
    ))
}

async fn flash_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<FlashRequest>,
) -> Result<Json<Value>, HttpError> {
    let (artifact, target, dry_run, evidence) = resolve_operation(
        &state,
        &id,
        input.target.clone(),
        input.artifact_id.clone(),
        input.dry_run.unwrap_or(true),
    )?;
    verify_artifact_files(&artifact)?;
    if dry_run {
        return Ok(Json(
            json!({"ok": true, "dry_run": true, "action": "flash", "target_evidence": evidence}),
        ));
    }
    enforce_flash_gate(&state, &id, &target, &artifact, &input)?;
    {
        let guard = state.inner.lock().expect("state lock");
        if target_requires_usb_lease(&target) {
            ensure_flash_lease_for_target(&guard, Some(&id), input.lease_id.as_deref(), &target)?;
        }
        let device = guard
            .devices
            .get(&id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        ensure_real_operation_uses_cached_target(device, &target)?;
    }
    let post_flash_identity = match target {
        TargetKind::DigitalEsp32s3 => {
            run_espflash_digital(&state, &id, &artifact).await?;
            let identity = capture_post_flash_identity(&state, &id).await?;
            if let Some(expected_identity) = input.expected_identity_device_id.as_deref() {
                let actual = identity.get("device_id").and_then(Value::as_str);
                if actual != Some(expected_identity) {
                    return Err(HttpError::conflict(
                        "post_flash_identity_mismatch",
                        format!(
                            "expected post-flash identity device_id {expected_identity}, current identity is {}",
                            actual.unwrap_or("<unknown>")
                        ),
                    ));
                }
            }
            Some(identity)
        }
        TargetKind::AnalogStm32g431 => {
            run_probe_rs_analog(&state, &id, &artifact).await?;
            None
        }
        TargetKind::LanHttp | TargetKind::Mock => {
            return Err(HttpError::bad_request(
                "target_unsupported",
                "target cannot be flashed",
            ));
        }
    };
    emit(
        &state,
        Some(id),
        "flash",
        "firmware flash completed",
        json!({"artifact_id": artifact.artifact_id, "target": target}),
    );
    Ok(Json(
        json!({"ok": true, "dry_run": false, "action": "flash", "target_evidence": evidence, "post_flash_identity": post_flash_identity}),
    ))
}

fn enforce_flash_gate(
    state: &AppState,
    device_id: &str,
    target: &TargetKind,
    artifact: &FirmwareArtifact,
    input: &FlashRequest,
) -> Result<(), HttpError> {
    if target == &TargetKind::AnalogStm32g431 {
        enforce_analog_operation_confirmation(input.confirmation_phrase.as_deref())?;
        enforce_non_project_firmware_acknowledgement(artifact, input)?;
        return Ok(());
    }
    if target != &TargetKind::DigitalEsp32s3 {
        return Ok(());
    }
    if !is_flash_confirmation(input.confirmation_phrase.as_deref()) {
        return Err(HttpError::bad_request(
            "flash_confirmation_required",
            format!("type `{FLASH_CONFIRMATION_TEXT}` to confirm real digital flash"),
        ));
    }
    enforce_non_project_firmware_acknowledgement(artifact, input)?;
    if let Some(expected_identity) = input.expected_identity_device_id.as_deref() {
        let guard = state.inner.lock().expect("state lock");
        let preflash_only_lease = input
            .lease_id
            .as_deref()
            .and_then(|lease_id| guard.leases.get(lease_id))
            .is_some_and(|lease| lease.legacy_preflash_only);
        let device = guard
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        let actual = device
            .identity
            .as_ref()
            .and_then(|identity| identity.get("device_id"))
            .and_then(Value::as_str);
        if actual != Some(expected_identity) && !preflash_only_lease {
            return Err(HttpError::conflict(
                "identity_confirmation_mismatch",
                format!(
                    "expected identity device_id {expected_identity}, current identity is {}",
                    actual.unwrap_or("<unknown>")
                ),
            ));
        }
    }
    Ok(())
}

fn enforce_non_project_firmware_acknowledgement(
    artifact: &FirmwareArtifact,
    input: &FlashRequest,
) -> Result<(), HttpError> {
    if !is_loadlynx_project_artifact(artifact)
        && input.acknowledge_non_project_firmware != Some(true)
    {
        return Err(HttpError::bad_request(
            "non_project_firmware_ack_required",
            "non-project or unknown firmware requires acknowledge_non_project_firmware=true",
        ));
    }
    Ok(())
}

fn enforce_analog_operation_confirmation(value: Option<&str>) -> Result<(), HttpError> {
    if is_flash_confirmation(value) {
        return Ok(());
    }
    Err(HttpError::bad_request(
        "operation_confirmation_required",
        format!("type `{FLASH_CONFIRMATION_TEXT}` to confirm real analog operation"),
    ))
}

fn is_flash_confirmation(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case(FLASH_CONFIRMATION_TEXT))
}

fn is_loadlynx_project_artifact(artifact: &FirmwareArtifact) -> bool {
    let haystack = format!(
        "{} {} {}",
        artifact.artifact_id, artifact.name, artifact.protocol
    )
    .to_ascii_lowercase();
    haystack.contains("loadlynx")
}

async fn capture_post_flash_identity(
    state: &AppState,
    device_id: &str,
) -> Result<Value, HttpError> {
    tokio::time::sleep(Duration::from_millis(750)).await;
    let port_path = {
        let guard = state.inner.lock().expect("state lock");
        let device = guard
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        device
            .digital_target
            .as_ref()
            .and_then(|target| target.port_path.clone())
            .ok_or_else(|| {
                HttpError::conflict(
                    "target_port_missing",
                    "post-flash identity capture requires the approved ESP32-S3 USB port path",
                )
            })?
    };
    let identity = request_usb_identity(state, device_id, &port_path).await?;
    update_device_identity(state, device_id, identity.clone())?;
    Ok(identity)
}

async fn reset_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<ResetRequest>>,
) -> Result<Json<Value>, HttpError> {
    let input = body.map(|Json(v)| v).unwrap_or(ResetRequest {
        target: None,
        dry_run: Some(true),
        lease_id: None,
        confirmation_phrase: None,
    });
    let target = input.target.unwrap_or(TargetKind::DigitalEsp32s3);
    let dry_run = input.dry_run.unwrap_or(true);
    let evidence = target_evidence(&state, &id, target.clone(), None)?;
    if dry_run {
        return Ok(Json(
            json!({"ok": true, "dry_run": true, "action": "reset", "target_evidence": evidence}),
        ));
    }
    if target == TargetKind::AnalogStm32g431 {
        enforce_analog_operation_confirmation(input.confirmation_phrase.as_deref())?;
    }
    {
        let guard = state.inner.lock().expect("state lock");
        if target_requires_usb_lease(&target) {
            ensure_lease_for_target(&guard, Some(&id), input.lease_id.as_deref())?;
        }
        let device = guard
            .devices
            .get(&id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        ensure_real_operation_uses_cached_target(device, &target)?;
    }
    match target {
        TargetKind::DigitalEsp32s3 => run_espflash_reset_digital(&state, &id).await?,
        TargetKind::AnalogStm32g431 => run_probe_rs_reset_analog(&state, &id).await?,
        TargetKind::LanHttp | TargetKind::Mock => {
            return Err(HttpError::bad_request(
                "target_unsupported",
                "target cannot be reset",
            ));
        }
    }
    emit(
        &state,
        Some(id),
        "reset",
        "device reset completed",
        json!({"target": target}),
    );
    Ok(Json(
        json!({"ok": true, "dry_run": false, "action": "reset", "target_evidence": evidence}),
    ))
}

async fn monitor_start(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let mut guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get_mut(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    device.connection = ConnectionState::Connected;
    push_log(
        device,
        "info",
        "monitor",
        "monitor started (bounded devd session)",
    );
    push_trace(
        device,
        "rx",
        json!({"type": "log", "level": "info", "message": "monitor started"}),
    );
    Ok(Json(json!({"ok": true, "monitor": "started"})))
}

async fn monitor_stop(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let mut guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get_mut(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    push_log(device, "info", "monitor", "monitor stopped");
    Ok(Json(json!({"ok": true, "monitor": "stopped"})))
}

async fn device_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    ensure_lease_for_target(&guard, Some(&id), query.lease_id.as_deref())?;
    let device = guard
        .devices
        .get(&id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    Ok(Json(session_json(
        device,
        query.logs_limit,
        query.trace_limit,
    )))
}

async fn create_lease(
    State(state): State<AppState>,
    Json(input): Json<LeaseRequest>,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    {
        let guard = state.inner.lock().expect("state lock");
        guard
            .devices
            .get(&input.device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    }
    let lease_port_path = device_digital_port(&state, &input.device_id);
    if let Some(port_path) = lease_port_path.as_deref() {
        if let Some(reason) = serial_exclusive_reason(&state, port_path) {
            return Err(HttpError::conflict(
                "operation_in_progress",
                format!("USB serial port is reserved for {reason}"),
            ));
        }
        if !port_path.starts_with("mock://") && input.bind_probe != Some(true) {
            match read_default_digital_usb_port(&state.repo_root) {
                Some(default) if default == port_path => {}
                _ => {
                    return Err(HttpError::conflict(
                        "target_selector_not_cached",
                        "USB lease requires the selected port to match the approved default digital USB port",
                    ));
                }
            }
        }
        validate_port_not_leased_by_other_device(&state, &input.device_id, port_path)?;
        if is_default_or_scanned_usb_source(&state, &input.device_id) {
            match request_usb_identity(&state, &input.device_id, port_path).await {
                Ok(identity) => {
                    if let Some(expected) = input.expected_identity_device_id.as_deref() {
                        let actual = identity.get("device_id").and_then(Value::as_str);
                        if actual != Some(expected) {
                            stop_serial_owner(&state, port_path);
                            return Err(HttpError::conflict(
                                "identity_confirmation_mismatch",
                                format!(
                                    "expected identity device_id {expected}, current identity is {}",
                                    actual.unwrap_or("<missing>")
                                ),
                            ));
                        }
                    }
                    update_device_identity_for_lease_probe(
                        &state,
                        &input.device_id,
                        port_path,
                        identity,
                    )?;
                }
                Err(error)
                    if port_path.starts_with("mock://")
                        || matches!(
                            error.0.code.as_str(),
                            "operation_in_progress" | "device_busy"
                        ) =>
                {
                    return Err(error);
                }
                Err(error) => {
                    let legacy_preflash_identity =
                        allows_legacy_preflash_identity_fallback(&input, &error);
                    let mut guard = state.inner.lock().expect("state lock");
                    if let Some(device) = guard.devices.get_mut(&input.device_id) {
                        push_log(
                            device,
                            "warn",
                            "serial",
                            &format!(
                                "serial identity probe failed for {port_path}: {}",
                                error.0.message
                            ),
                        );
                    }
                    drop(guard);
                    stop_serial_owner(&state, port_path);
                    if legacy_preflash_identity {
                        update_device_identity_for_lease_probe(
                            &state,
                            &input.device_id,
                            port_path,
                            json!({
                                "device_id": "digital-esp32s3",
                                "target": "digital",
                                "mcu": "esp32s3",
                                "protocol": "loadlynx.cdc.v1",
                                "legacy_preflash_identity_fallback": true,
                                "identity_probe_error_code": error.0.code,
                                "identity_probe_error": error.0.message,
                            }),
                        )?;
                    } else {
                        return Err(error);
                    }
                }
            }
        }
    }
    let _ = connect_device(State(state.clone()), Path(input.device_id.clone()), None).await?;

    let lease_id = next_id();
    let legacy_preflash_only = {
        let guard = state.inner.lock().expect("state lock");
        guard
            .devices
            .get(&input.device_id)
            .and_then(|d| d.identity.as_ref())
            .and_then(|i| i.get("legacy_preflash_identity_fallback"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let identity_device_id = {
        let guard = state.inner.lock().expect("state lock");
        guard
            .devices
            .get(&input.device_id)
            .and_then(|d| d.identity.as_ref())
            .and_then(|i| i.get("device_id"))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let lease = WebLease {
        lease_id: lease_id.clone(),
        device_id: input.device_id.clone(),
        identity_device_id,
        bind_probe: input.bind_probe == Some(true),
        legacy_preflash_only,
        port_path: lease_port_path,
        expires_at: Instant::now() + Duration::from_millis(WEB_LEASE_TTL_MS),
    };
    state
        .inner
        .lock()
        .expect("state lock")
        .leases
        .insert(lease_id.clone(), lease.clone());
    emit(
        &state,
        Some(input.device_id),
        "web_lease",
        "Web USB lease created",
        json!({"lease_id": lease_id}),
    );
    Ok(Json(json!({
        "lease_id": lease_id,
        "device_id": lease.device_id,
        "identity_device_id": lease.identity_device_id,
        "heartbeat_interval_ms": WEB_LEASE_HEARTBEAT_INTERVAL_MS,
        "lease_ttl_ms": WEB_LEASE_TTL_MS
    })))
}

fn allows_legacy_preflash_identity_fallback(input: &LeaseRequest, error: &HttpError) -> bool {
    input.allow_legacy_preflash_identity_fallback == Some(true)
        && input
            .expected_identity_device_id
            .as_deref()
            .is_some_and(|id| id == "digital-esp32s3" || is_stable_identity_device_id(id))
        && matches!(
            error.0.code.as_str(),
            "serial_response_timeout" | "serial_response_missing" | "serial_response_invalid"
        )
}

fn is_stable_identity_device_id(id: &str) -> bool {
    id.strip_prefix("loadlynx-").is_some_and(|short_id| {
        short_id.len() == 6
            && short_id
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    }) || id.starts_with("mock-")
}

async fn heartbeat_lease(
    State(state): State<AppState>,
    Path(lease_id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let mut guard = state.inner.lock().expect("state lock");
    let lease = guard
        .leases
        .get_mut(&lease_id)
        .ok_or_else(|| HttpError::bad_request("web_session_expired", "Web USB lease is expired"))?;
    lease.expires_at = Instant::now() + Duration::from_millis(WEB_LEASE_TTL_MS);
    Ok(Json(json!({
        "lease_id": lease.lease_id,
        "device_id": lease.device_id,
        "identity_device_id": lease.identity_device_id,
        "heartbeat_interval_ms": WEB_LEASE_HEARTBEAT_INTERVAL_MS,
        "lease_ttl_ms": WEB_LEASE_TTL_MS
    })))
}

async fn release_lease(
    State(state): State<AppState>,
    Path(lease_id): Path<String>,
) -> Result<Json<Value>, HttpError> {
    let released = release_lease_inner(&state, &lease_id, "released");
    Ok(Json(json!({"ok": true, "released": released})))
}

async fn compat_session(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = select_compat_device(&guard, query.lease_id.as_deref(), None)?;
    Ok(Json(session_json(
        device,
        query.logs_limit,
        query.trace_limit,
    )))
}

async fn serial_owner_jsonl_request(
    state: &AppState,
    device_id: &str,
    port_path: &str,
    op: &str,
    extra: Option<Value>,
) -> Result<(String, SerialProtocolProbe), HttpError> {
    let request_id = next_request_id(op);
    if port_path.starts_with("mock://") {
        return Ok((
            request_id.clone(),
            mock_serial_probe(state, &request_id, op, extra),
        ));
    }
    if let Some(reason) = serial_exclusive_reason(state, port_path) {
        return Err(HttpError::conflict(
            "operation_in_progress",
            format!("USB serial port is reserved for {reason}"),
        ));
    }

    let operation_wait_ms = serial_operation_wait_ms(op, extra.as_ref());
    let command = SerialJsonlCommand {
        device_id: device_id.to_string(),
        port_path: port_path.to_string(),
        request_id: request_id.clone(),
        op: op.to_string(),
        extra,
    };
    let (tx, rx) = oneshot::channel();
    let sender = serial_owner_sender(state, port_path)?;
    sender
        .try_send(SerialWorkerCommand {
            request: command,
            reply: tx,
        })
        .map_err(|error| match error {
            std_mpsc::TrySendError::Full(_) => {
                HttpError::conflict("device_busy", "USB serial operation queue is full")
            }
            std_mpsc::TrySendError::Disconnected(_) => HttpError::retryable(
                "serial_owner_stopped",
                "USB serial owner stopped before accepting the request",
            ),
        })?;
    let result = tokio::time::timeout(Duration::from_millis(operation_wait_ms), rx)
        .await
        .map_err(|_| {
            HttpError::retryable(
                "serial_operation_timeout",
                format!("USB serial operation {request_id} timed out waiting for owner"),
            )
        })?
        .map_err(|_| {
            HttpError::retryable(
                "serial_owner_stopped",
                "USB serial owner stopped before replying",
            )
        })?;

    match result {
        Ok(probe) => Ok((request_id, probe)),
        Err(error) => Err(HttpError(
            ApiError {
                code: error.code.to_string(),
                message: error.message,
                retryable: error.retryable,
                details: None,
            },
            if error.retryable {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::CONFLICT
            },
        )),
    }
}

fn serial_owner_sender(
    state: &AppState,
    port_path: &str,
) -> Result<std_mpsc::SyncSender<SerialWorkerCommand>, HttpError> {
    let port_key = canonical_port_key(port_path);
    let mut registry = state.serial.lock().expect("serial registry lock");
    if let Some(reason) = registry.exclusive_ports.get(&port_key) {
        return Err(HttpError::conflict(
            "operation_in_progress",
            format!("USB serial port is reserved for {reason}"),
        ));
    }
    if let Some(owner) = registry.owners.get(&port_key) {
        return Ok(owner.tx.clone());
    }

    let (tx, rx) = std_mpsc::sync_channel(SERIAL_COMMAND_QUEUE_LIMIT);
    registry.next_owner_id += 1;
    let owner_id = registry.next_owner_id;
    let worker_state = state.clone();
    let worker_port = port_path.to_string();
    let join = thread::Builder::new()
        .name(format!(
            "loadlynx-serial-{}",
            sanitize_thread_name(&port_key)
        ))
        .spawn(move || serial_worker_loop(worker_state, worker_port, owner_id, rx))
        .expect("spawn serial worker");
    registry.owners.insert(
        port_key,
        SerialOwnerHandle {
            id: owner_id,
            tx: tx.clone(),
            join,
        },
    );
    Ok(tx)
}

fn stop_serial_owner(state: &AppState, port_path: &str) {
    let port_key = canonical_port_key(port_path);
    let owner = state
        .serial
        .lock()
        .expect("serial registry lock")
        .owners
        .remove(&port_key);
    if let Some(owner) = owner {
        drop(owner.tx);
        let _ = owner.join.join();
    }
}

fn serial_exclusive_reason(state: &AppState, port_path: &str) -> Option<String> {
    let port_key = canonical_port_key(port_path);
    state
        .serial
        .lock()
        .expect("serial registry lock")
        .exclusive_ports
        .get(&port_key)
        .cloned()
}

fn mark_serial_exclusive(state: &AppState, port_path: &str, reason: &str) -> Result<(), HttpError> {
    let port_key = canonical_port_key(port_path);
    let mut registry = state.serial.lock().expect("serial registry lock");
    if let Some(existing) = registry.exclusive_ports.get(&port_key) {
        return Err(HttpError::conflict(
            "operation_in_progress",
            format!("USB serial port is already reserved for {existing}"),
        ));
    }
    let owner = registry.owners.remove(&port_key);
    registry
        .exclusive_ports
        .insert(port_key, reason.to_string());
    drop(registry);
    if let Some(owner) = owner {
        drop(owner.tx);
        let _ = owner.join.join();
    }
    Ok(())
}

fn clear_serial_exclusive(state: &AppState, port_path: &str) {
    let port_key = canonical_port_key(port_path);
    state
        .serial
        .lock()
        .expect("serial registry lock")
        .exclusive_ports
        .remove(&port_key);
}

fn clear_serial_exclusive_and_resume_owner(state: &AppState, port_path: &str) {
    clear_serial_exclusive(state, port_path);
    if has_active_lease_for_port(state, port_path) {
        let _ = serial_owner_sender(state, port_path);
    }
}

fn reserve_serial_exclusive(
    state: &AppState,
    port_path: &str,
    reason: &str,
) -> Result<SerialExclusiveGuard, HttpError> {
    mark_serial_exclusive(state, port_path, reason)?;
    Ok(SerialExclusiveGuard {
        state: state.clone(),
        port_path: port_path.to_string(),
    })
}

async fn compat_identity(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let (device_id, port_path) = {
        let guard = state.inner.lock().expect("state lock");
        select_serial_port_for_compat(&guard, &query, "identity")?
    };

    let identity = request_usb_identity(&state, &device_id, &port_path).await?;
    if let Some(device) = state
        .inner
        .lock()
        .expect("state lock")
        .devices
        .get_mut(&device_id)
    {
        device.identity = Some(identity.clone());
    }
    Ok(Json(identity))
}

async fn request_usb_identity(
    state: &AppState,
    device_id: &str,
    port_path: &str,
) -> Result<Value, HttpError> {
    let mut last_error = None;
    for attempt in 0..2 {
        let (request_id, probe) =
            match serial_owner_jsonl_request(state, device_id, port_path, "get_identity", None)
                .await
            {
                Ok(result) => result,
                Err(error) if error.0.retryable && attempt == 0 => {
                    last_error = Some(error);
                    continue;
                }
                Err(error) => return Err(error),
            };
        let response = serial_response_for_request(&probe, &request_id)
            .or_else(|| infer_serial_response_from_fragments(&probe, &request_id))
            .or_else(|| infer_serial_response_from_text(&probe, &request_id));

        record_serial_protocol_probe(
            state,
            device_id,
            port_path,
            if attempt == 0 {
                "USB identity request completed"
            } else {
                "USB identity retry completed"
            },
            probe,
        );
        match identity_data_from_serial_response(response) {
            Ok(identity) => return Ok(identity),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            "USB identity did not return a protocol response",
        )
    }))
}

fn is_retryable_serial_gap_or_shape_error(error: &HttpError) -> bool {
    matches!(
        error.0.code.as_str(),
        "serial_response_timeout"
            | "serial_response_mismatch"
            | "serial_response_missing"
            | "serial_response_invalid"
    )
}

async fn request_compat_status_data(
    state: &AppState,
    device_id: &str,
    port_path: &str,
) -> Result<Value, HttpError> {
    let mut last_error = None;
    for attempt in 0..3 {
        let (request_id, probe) =
            match serial_owner_jsonl_request(state, device_id, port_path, "get_status", None).await
            {
                Ok(result) => result,
                Err(error) if attempt < 2 && is_retryable_serial_gap_or_shape_error(&error) => {
                    last_error = Some(error);
                    continue;
                }
                Err(error) => return Err(error),
            };
        let response = serial_response_for_request(&probe, &request_id)
            .or_else(|| infer_serial_response_from_fragments(&probe, &request_id))
            .or_else(|| infer_serial_response_from_text(&probe, &request_id));
        record_serial_protocol_probe(
            state,
            device_id,
            port_path,
            if attempt == 0 {
                "USB status request completed"
            } else {
                "USB status retry completed"
            },
            probe,
        );
        match status_data_from_serial_response(response) {
            Ok(data) => {
                if let Some(device) = state
                    .inner
                    .lock()
                    .expect("state lock")
                    .devices
                    .get_mut(device_id)
                {
                    let _ = maybe_update_device_status_cache(device, &data);
                    let _ = maybe_update_device_control_cache(device, &data);
                    return Ok(merge_cached_control_if_missing(device, data));
                }
                return Ok(data);
            }
            Err(error) if attempt < 2 && is_retryable_serial_gap_or_shape_error(&error) => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            "USB status did not return a protocol response",
        )
    }))
}

async fn compat_status(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let (device_id, port_path) = {
        let guard = state.inner.lock().expect("state lock");
        select_serial_port_for_compat(&guard, &query, "status")?
    };

    if query.cache && !query.fresh {
        if let Some(cached) = fresh_cached_status_data(&state, &device_id) {
            return Ok(Json(finalize_status_output(&device_id, cached)?));
        }
        if let Some(cached) =
            cached_usb_status_output(&state, &device_id, SERIAL_STATUS_CACHE_MAX_AGE_MS)
        {
            return Ok(Json(cached));
        }
    }

    let data = request_compat_status_data(&state, &device_id, &port_path).await?;
    let output = finalize_status_output(&device_id, data)?;
    update_usb_status_cache(&state, &device_id, output, "compat_status_request");
    Ok(Json(
        cached_usb_status_output(&state, &device_id, -1).ok_or_else(|| {
            HttpError::retryable("status_cache_missing", "USB status cache was not updated")
        })?,
    ))
}

async fn compat_network(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = select_compat_device(
        &guard,
        query.lease_id.as_deref(),
        query.device_id.as_deref(),
    )?;
    Ok(Json(json!({"lan_endpoint": device.lan_endpoint})))
}

async fn compat_cc(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    Json(input): Json<CcRequest>,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let (device_id, port_path) = {
        let guard = state.inner.lock().expect("state lock");
        let device = select_compat_device(
            &guard,
            query.lease_id.as_deref(),
            query.device_id.as_deref(),
        )?;
        ensure_active_lease_for_device(&guard, &device.id, query.lease_id.as_deref(), false)?;
        let port_path = device
            .digital_target
            .as_ref()
            .and_then(|target| target.port_path.clone())
            .ok_or_else(|| {
                HttpError::conflict(
                    "target_port_missing",
                    "USB CC control requires a selected digital USB port",
                )
            })?;
        (device.id.clone(), port_path)
    };

    let (request_id, probe) = serial_owner_jsonl_request(
        &state,
        &device_id,
        &port_path,
        "set_output_enabled",
        Some(match input.target_i_ma {
            Some(target_i_ma) => json!({"enable": input.enable, "target_i_ma": target_i_ma}),
            None => json!({"enable": input.enable}),
        }),
    )
    .await?;
    let response = probe
        .frames
        .iter()
        .rev()
        .find(|event| {
            event.direction == "rx"
                && event
                    .frame
                    .get("request_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == request_id)
        })
        .map(|event| event.frame.clone())
        .or_else(|| infer_serial_response_from_fragments(&probe, &request_id))
        .or_else(|| infer_serial_response_from_text(&probe, &request_id));

    record_serial_protocol_probe(
        &state,
        &device_id,
        &port_path,
        "USB CC control request completed",
        probe,
    );
    let response = response.ok_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            "USB CC control did not return a protocol response",
        )
    })?;
    let _ = serial_response_data(response.clone(), "USB CC control")?;
    Ok(Json(json!({
        "ok": true,
        "request_id": request_id,
        "response": response
    })))
}

async fn compat_usb_json_request(
    state: &AppState,
    query: &CompatQuery,
    op: &str,
    extra: Option<Value>,
    success_message: &str,
    operation: &str,
) -> Result<(String, Value), HttpError> {
    cleanup_expired_leases(state);
    let (device_id, port_path) = {
        let guard = state.inner.lock().expect("state lock");
        select_serial_port_for_compat(&guard, query, operation)?
    };

    let (request_id, probe) =
        serial_owner_jsonl_request(state, &device_id, &port_path, op, extra).await?;
    let response = serial_response_for_request(&probe, &request_id)
        .or_else(|| infer_serial_response_from_fragments(&probe, &request_id))
        .or_else(|| infer_serial_response_from_text(&probe, &request_id));
    record_serial_protocol_probe(state, &device_id, &port_path, success_message, probe);
    Ok((
        request_id,
        serial_response_data_required(response, operation)?,
    ))
}

async fn compat_usb_json_request_with_retry(
    state: &AppState,
    query: &CompatQuery,
    op: &str,
    extra: Option<Value>,
    success_message: &str,
    operation: &str,
    max_attempts: usize,
) -> Result<(String, Value), HttpError> {
    let mut last_error = None;
    for attempt in 0..max_attempts.max(1) {
        match compat_usb_json_request(
            state,
            query,
            op,
            extra.clone(),
            if attempt == 0 {
                success_message
            } else {
                "USB compat request retry completed"
            },
            operation,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(error)
                if attempt + 1 < max_attempts && is_retryable_serial_gap_or_shape_error(&error) =>
            {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            format!("{operation} did not return a protocol response"),
        )
    }))
}

async fn compat_wifi_get(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "get_wifi_status",
        None,
        "USB WiFi status completed",
        "USB WiFi status",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_wifi_credentials_get(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "get_wifi_credentials",
        None,
        "USB WiFi credentials completed",
        "USB WiFi credentials",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_wifi_post(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    Json(input): Json<WifiSetRequest>,
) -> Result<Json<Value>, HttpError> {
    let extra = json!({
        "ssid": input.ssid,
        "psk": input.psk,
        "wait": input.wait,
    });
    let (request_id, data) = compat_usb_json_request(
        &state,
        &query,
        "set_wifi_config",
        Some(extra),
        "USB WiFi set completed",
        "USB WiFi set",
    )
    .await?;
    Ok(Json(json!({"request_id": request_id, "wifi": data})))
}

async fn compat_wifi_delete(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let (request_id, data) = compat_usb_json_request(
        &state,
        &query,
        "clear_wifi_config",
        None,
        "USB WiFi clear completed",
        "USB WiFi clear",
    )
    .await?;
    Ok(Json(json!({"request_id": request_id, "wifi": data})))
}

async fn compat_control_get(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let (request_id, data) = compat_usb_json_request_with_retry(
        &state,
        &query,
        "get_control",
        None,
        "USB control GET completed",
        "USB control GET",
        3,
    )
    .await?;
    if let Some(device_id) = query.device_id.as_deref()
        && let Some(device) = state
            .inner
            .lock()
            .expect("state lock")
            .devices
            .get_mut(device_id)
    {
        let _ = maybe_update_device_control_cache(device, &data);
        push_trace(
            device,
            "rx",
            json!({
                "type": "control_cache_refresh",
                "request_id": request_id,
                "cached": true
            }),
        );
    }
    Ok(Json(data))
}

async fn compat_control_post(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "set_control",
        Some(input),
        "USB control SET completed",
        "USB control SET",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_presets_get(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let mut merged = HashMap::new();
    let mut recovered = false;
    let mut recovered_by_control = false;
    let mut last_error = None;
    for attempt in 0..3 {
        match compat_usb_json_request(
            &state,
            &query,
            "get_presets",
            None,
            if attempt == 0 {
                "USB presets GET completed"
            } else {
                "USB presets GET retry completed"
            },
            "USB presets GET",
        )
        .await
        {
            Ok((_, data)) => {
                if !merge_presets_from_data(&mut merged, &data) {
                    return Ok(Json(data));
                }
                recovered |= data
                    .get("recovered_from_fragments")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    || data
                        .get("recovered_from_text")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                if merged.len() >= LOADLYNX_PRESET_COUNT {
                    return Ok(Json(presets_data_from_map(
                        &merged,
                        recovered,
                        recovered_by_control,
                    )));
                }
            }
            Err(error) => last_error = Some(error),
        }
    }
    if !merged.is_empty() {
        for attempt in 0..3 {
            if let Ok((_, data)) = compat_usb_json_request(
                &state,
                &query,
                "get_control",
                None,
                if attempt == 0 {
                    "USB presets GET control recovery completed"
                } else {
                    "USB presets GET control recovery retry completed"
                },
                "USB presets GET control recovery",
            )
            .await
                && merge_presets_from_data(&mut merged, &data)
            {
                recovered_by_control = true;
                recovered |= data
                    .get("recovered_from_fragments")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    || data
                        .get("recovered_from_text")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                if merged.len() >= LOADLYNX_PRESET_COUNT {
                    return Ok(Json(presets_data_from_map(
                        &merged,
                        recovered,
                        recovered_by_control,
                    )));
                }
            }
        }
    }
    if !merged.is_empty() {
        return Err(HttpError::retryable(
            "serial_response_incomplete",
            format!(
                "USB presets GET recovered {}/{} presets after retries",
                merged.len(),
                LOADLYNX_PRESET_COUNT
            ),
        ));
    }
    Err(last_error.unwrap_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            "USB presets GET did not return a protocol response",
        )
    }))
}

async fn compat_presets_post(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "set_preset",
        Some(input),
        "USB preset SET completed",
        "USB preset SET",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_presets_apply(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "apply_preset",
        Some(input),
        "USB preset APPLY completed",
        "USB preset APPLY",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_calibration_profile(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "get_calibration_profile",
        None,
        "USB calibration profile completed",
        "USB calibration profile",
    )
    .await?;
    Ok(Json(expand_compact_calibration_profile(data)?))
}

async fn compat_calibration_apply(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "calibration_apply",
        Some(input),
        "USB calibration apply completed",
        "USB calibration apply",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_calibration_commit(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "calibration_commit",
        Some(input),
        "USB calibration commit completed",
        "USB calibration commit",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_calibration_reset(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "calibration_reset",
        Some(input),
        "USB calibration reset completed",
        "USB calibration reset",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_calibration_mode(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "calibration_mode",
        Some(input),
        "USB calibration mode completed",
        "USB calibration mode",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_soft_reset(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    let input = parse_compat_json_body(&body)?;
    let (_, data) = compat_usb_json_request(
        &state,
        &query,
        "soft_reset",
        Some(input),
        "USB soft reset completed",
        "USB soft reset",
    )
    .await?;
    Ok(Json(data))
}

async fn compat_diagnostics_export(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Value>, HttpError> {
    let compat_query = CompatQuery {
        device_id: query.device_id.clone(),
        lease_id: query.lease_id.clone(),
        fresh: false,
        cache: false,
    };
    let (_, firmware) = compat_usb_json_request(
        &state,
        &compat_query,
        "get_diagnostics",
        None,
        "USB diagnostics export completed",
        "USB diagnostics export",
    )
    .await?;

    cleanup_expired_leases(&state);
    let guard = state.inner.lock().expect("state lock");
    let device = select_compat_device(
        &guard,
        query.lease_id.as_deref(),
        query.device_id.as_deref(),
    )?;
    let leases = guard
        .leases
        .values()
        .filter(|lease| lease.device_id == device.id)
        .map(lease_json)
        .collect::<Vec<_>>();
    let events = guard
        .events
        .iter()
        .filter(|event| event.device_id.as_deref().is_none_or(|id| id == device.id))
        .map(redact_sensitive_event)
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "schema_version": 1,
        "exported_at": now(),
        "device": redact_sensitive_frame(&json!(device)),
        "firmware": redact_sensitive_frame(&firmware),
        "leases": leases,
        "session": session_json(device, query.logs_limit, query.trace_limit),
        "events": events,
        "redaction": {
            "psk": true,
            "password": true
        }
    })))
}

fn redact_sensitive_event(event: &DevdEvent) -> DevdEvent {
    DevdEvent {
        id: event.id.clone(),
        timestamp: event.timestamp.clone(),
        device_id: event.device_id.clone(),
        kind: event.kind.clone(),
        message: event.message.clone(),
        payload: redact_sensitive_frame(&event.payload),
    }
}

fn parse_compat_json_body(body: &str) -> Result<Value, HttpError> {
    serde_json::from_str(body)
        .map_err(|error| HttpError::bad_request("invalid_request", error.to_string()))
}

fn select_serial_port_for_compat(
    state: &DevdState,
    query: &CompatQuery,
    operation: &str,
) -> Result<(String, String), HttpError> {
    let device =
        select_compat_device(state, query.lease_id.as_deref(), query.device_id.as_deref())?;
    ensure_active_lease_for_device(
        state,
        &device.id,
        query.lease_id.as_deref(),
        operation == "identity",
    )?;
    let port_path = device
        .digital_target
        .as_ref()
        .and_then(|target| target.port_path.clone())
        .ok_or_else(|| {
            HttpError::conflict(
                "target_port_missing",
                format!("USB {operation} requires a selected digital USB port"),
            )
        })?;
    Ok((device.id.clone(), port_path))
}

fn cached_pd_view(state: &AppState, device_id: &str) -> Option<Value> {
    state
        .inner
        .lock()
        .expect("state lock")
        .devices
        .get(device_id)
        .and_then(|device| device.usb_pd_cache.clone())
}

async fn compat_pd_get(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let (device_id, port_path) = {
        let guard = state.inner.lock().expect("state lock");
        select_serial_port_for_compat(&guard, &query, "PD control")?
    };

    let (request_id, probe) =
        match serial_owner_jsonl_request(&state, &device_id, &port_path, "get_pd", None).await {
            Ok(result) => result,
            Err(error) => return pd_cache_or_serial_error(&state, &device_id, error),
        };
    let response = probe
        .frames
        .iter()
        .rev()
        .find(|event| {
            event.direction == "rx"
                && event
                    .frame
                    .get("request_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == request_id)
        })
        .map(|event| event.frame.clone());
    record_serial_protocol_probe(
        &state,
        &device_id,
        &port_path,
        "USB PD GET completed",
        probe,
    );
    if let Some(response) = response {
        return Ok(Json(pd_response_data(response)?));
    }
    if let Some(cached) = cached_pd_view(&state, &device_id) {
        return Ok(Json(cached));
    }
    Err(HttpError::retryable(
        "serial_response_missing",
        "USB PD GET did not return a protocol response",
    ))
}

fn pd_cache_or_serial_error(
    state: &AppState,
    device_id: &str,
    error: HttpError,
) -> Result<Json<Value>, HttpError> {
    if is_serial_response_gap_error(&error)
        && let Some(cached) = cached_pd_view(state, device_id)
    {
        return Ok(Json(cached));
    }
    Err(error)
}

fn is_serial_response_gap_error(error: &HttpError) -> bool {
    matches!(
        error.0.code.as_str(),
        "serial_response_timeout" | "serial_response_mismatch"
    )
}

async fn compat_pd_post(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
    body: String,
) -> Result<Json<Value>, HttpError> {
    cleanup_expired_leases(&state);
    let input: PdRequest = serde_json::from_str(&body)
        .map_err(|error| HttpError::bad_request("invalid_request", error.to_string()))?;
    let (device_id, port_path) = {
        let guard = state.inner.lock().expect("state lock");
        select_serial_port_for_compat(&guard, &query, "PD control")?
    };

    let mut extra = serde_json::Map::new();
    if let Some(mode) = input.mode {
        extra.insert("mode".to_string(), Value::String(mode));
    }
    if let Some(object_pos) = input.object_pos {
        extra.insert("object_pos".to_string(), json!(object_pos));
    }
    if let Some(target_mv) = input.target_mv {
        extra.insert("target_mv".to_string(), json!(target_mv));
    }
    if let Some(i_req_ma) = input.i_req_ma {
        extra.insert("i_req_ma".to_string(), json!(i_req_ma));
    }
    if let Some(allow_extended_voltage) = input.allow_extended_voltage {
        extra.insert(
            "allow_extended_voltage".to_string(),
            json!(allow_extended_voltage),
        );
    }

    let (request_id, probe) = serial_owner_jsonl_request(
        &state,
        &device_id,
        &port_path,
        "set_pd_policy",
        Some(Value::Object(extra)),
    )
    .await?;
    let response = probe
        .frames
        .iter()
        .rev()
        .find(|event| {
            event.direction == "rx"
                && event
                    .frame
                    .get("request_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == request_id)
        })
        .map(|event| event.frame.clone());
    record_serial_protocol_probe(
        &state,
        &device_id,
        &port_path,
        "USB PD POST completed",
        probe,
    );
    Ok(Json(pd_post_response_data(response)?))
}

async fn compat_events(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>>, HttpError>
{
    let device_id = {
        let guard = state.inner.lock().expect("state lock");
        select_compat_device(&guard, query.lease_id.as_deref(), None)?
            .id
            .clone()
    };
    Ok(events_stream(state, Some(device_id)))
}

async fn device_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SessionQuery>,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>>, HttpError>
{
    {
        let guard = state.inner.lock().expect("state lock");
        ensure_lease_for_target(&guard, Some(&id), query.lease_id.as_deref())?;
    }
    Ok(events_stream(state, Some(id)))
}

fn events_stream(
    state: AppState,
    device_id: Option<String>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.events.subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if device_id.as_ref().is_none_or(|id| event.device_id.as_ref() == Some(id)) {
                        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                        yield Ok(Event::default().event(event.kind).data(data));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream)
}

fn select_compat_device<'a>(
    state: &'a DevdState,
    lease_id: Option<&str>,
    device_id: Option<&str>,
) -> Result<&'a DeviceRecord, HttpError> {
    if let Some(device_id) = device_id {
        return state
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"));
    }
    if let Some(lease_id) = lease_id {
        let lease = active_lease_by_id(state, lease_id)?;
        return state
            .devices
            .get(&lease.device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "leased device is not known"));
    }
    let mut selected_device_id = None;
    for lease in state
        .leases
        .values()
        .filter(|lease| lease.expires_at > Instant::now())
    {
        match selected_device_id {
            None => selected_device_id = Some(lease.device_id.as_str()),
            Some(id) if id == lease.device_id => {}
            Some(_) => {
                return Err(HttpError::bad_request(
                    "device_selection_required",
                    "multiple Web USB leases are active; specify lease_id or device_id",
                ));
            }
        }
    }
    let Some(device_id) = selected_device_id else {
        return Err(HttpError::bad_request(
            "web_session_required",
            "Web USB lease or explicit device_id is required",
        ));
    };
    state
        .devices
        .get(device_id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "leased device is not known"))
}

fn ensure_lease_for_target(
    state: &DevdState,
    target_device_id: Option<&str>,
    lease_id: Option<&str>,
) -> Result<(), HttpError> {
    let Some(lease_id) = lease_id else {
        return Err(HttpError::bad_request(
            "web_session_required",
            "Web USB lease is required for devd USB session access",
        ));
    };
    let lease = active_lease_by_id(state, lease_id)?;
    ensure_operation_lease(lease)?;
    if let Some(target) = target_device_id
        && lease.device_id != target
    {
        return Err(HttpError::conflict(
            "device_lease_mismatch",
            "Web USB lease does not match requested device",
        ));
    }
    Ok(())
}

fn ensure_active_lease_for_device(
    state: &DevdState,
    device_id: &str,
    lease_id: Option<&str>,
    allow_bind_probe: bool,
) -> Result<(), HttpError> {
    if let Some(lease_id) = lease_id {
        let lease = active_lease_by_id(state, lease_id)?;
        if !allow_bind_probe {
            ensure_operation_lease(lease)?;
        }
        if lease.device_id != device_id {
            return Err(HttpError::conflict(
                "device_lease_mismatch",
                "Web USB lease does not match requested device",
            ));
        }
        return Ok(());
    }

    let active = state
        .leases
        .values()
        .filter(|lease| {
            lease.device_id == device_id
                && lease.expires_at > Instant::now()
                && (allow_bind_probe || !lease.bind_probe)
                && !lease.legacy_preflash_only
        })
        .collect::<Vec<_>>();
    if active.is_empty() {
        Err(HttpError::bad_request(
            "web_session_required",
            "Web USB lease is required for devd USB session access",
        ))
    } else {
        Ok(())
    }
}

fn ensure_operation_lease(lease: &WebLease) -> Result<(), HttpError> {
    if lease.bind_probe {
        return Err(HttpError::conflict(
            "bind_probe_lease_restricted",
            "bind-probe lease can only be used for identity binding",
        ));
    }
    if lease.legacy_preflash_only {
        return Err(HttpError::conflict(
            "legacy_preflash_lease_restricted",
            "legacy preflash lease can only be used for digital flash",
        ));
    }
    Ok(())
}

fn ensure_flash_lease_for_target(
    state: &DevdState,
    target_device_id: Option<&str>,
    lease_id: Option<&str>,
    target: &TargetKind,
) -> Result<(), HttpError> {
    let Some(lease_id) = lease_id else {
        return Err(HttpError::bad_request(
            "web_session_required",
            "Web USB lease is required for devd USB session access",
        ));
    };
    let lease = active_lease_by_id(state, lease_id)?;
    if lease.bind_probe {
        return Err(HttpError::conflict(
            "bind_probe_lease_restricted",
            "bind-probe lease can only be used for identity binding",
        ));
    }
    if lease.legacy_preflash_only && target != &TargetKind::DigitalEsp32s3 {
        return Err(HttpError::conflict(
            "legacy_preflash_lease_restricted",
            "legacy preflash lease can only be used for digital flash",
        ));
    }
    if let Some(target_device_id) = target_device_id
        && lease.device_id != target_device_id
    {
        return Err(HttpError::conflict(
            "device_lease_mismatch",
            "Web USB lease does not match requested device",
        ));
    }
    Ok(())
}

fn active_lease_by_id<'a>(state: &'a DevdState, lease_id: &str) -> Result<&'a WebLease, HttpError> {
    state
        .leases
        .get(lease_id)
        .filter(|lease| lease.expires_at > Instant::now())
        .ok_or_else(|| HttpError::bad_request("web_session_expired", "Web USB lease is expired"))
}

fn release_lease_inner(state: &AppState, lease_id: &str, reason: &str) -> bool {
    let lease = state
        .inner
        .lock()
        .expect("state lock")
        .leases
        .remove(lease_id);
    let Some(lease) = lease else {
        return false;
    };
    let port_path = lease
        .port_path
        .clone()
        .or_else(|| device_digital_port(state, &lease.device_id));
    let should_disconnect = {
        let guard = state.inner.lock().expect("state lock");
        !guard
            .leases
            .values()
            .any(|item| item.device_id == lease.device_id && item.expires_at > Instant::now())
    };
    if should_disconnect {
        let _ = disconnect_device_inner(state, &lease.device_id);
    }
    if let Some(port_path) = port_path
        && !has_active_lease_for_port(state, &port_path)
    {
        stop_serial_owner(state, &port_path);
    }
    emit(
        state,
        Some(lease.device_id),
        "web_lease",
        &format!("Web USB lease {reason}"),
        json!({"lease_id": lease_id}),
    );
    true
}

fn has_active_lease_for_port(state: &AppState, port_path: &str) -> bool {
    let port_key = canonical_port_key(port_path);
    let guard = state.inner.lock().expect("state lock");
    guard.leases.values().any(|lease| {
        if lease.expires_at <= Instant::now() {
            return false;
        }
        lease
            .port_path
            .as_deref()
            .or_else(|| {
                guard
                    .devices
                    .get(&lease.device_id)
                    .and_then(|device| device.digital_target.as_ref())
                    .and_then(|target| target.port_path.as_deref())
            })
            .is_some_and(|other| canonical_port_key(other) == port_key)
    })
}

fn spawn_lease_reaper(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1_000));
        loop {
            interval.tick().await;
            cleanup_expired_leases(&state);
        }
    });
}

fn cleanup_expired_leases(state: &AppState) {
    let expired = {
        let guard = state.inner.lock().expect("state lock");
        guard
            .leases
            .iter()
            .filter(|(_, lease)| lease.expires_at <= Instant::now())
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
    };
    for id in expired {
        release_lease_inner(state, &id, "expired");
    }
}

fn session_json(
    device: &DeviceRecord,
    logs_limit: Option<usize>,
    trace_limit: Option<usize>,
) -> Value {
    json!({
        "connected": device.connection == ConnectionState::Connected,
        "log_decode": device.log_decode,
        "status_cache": device.status_cache.clone(),
        "control_cache": device.control_cache.clone(),
        "status_meta_cache": device.status_meta_cache.clone(),
        "status_cache_updated_at_ms": device.status_cache_updated_at_ms,
        "logs": tail(&device.logs, logs_limit.unwrap_or(200).min(LOG_LIMIT)),
        "trace": tail(&device.trace, trace_limit.unwrap_or(600).min(TRACE_LIMIT)),
    })
}

fn lease_json(lease: &WebLease) -> Value {
    json!({
        "lease_id": lease.lease_id,
        "device_id": lease.device_id,
        "identity_device_id": lease.identity_device_id,
        "bind_probe": lease.bind_probe,
        "legacy_preflash_only": lease.legacy_preflash_only,
        "lease_ttl_ms": WEB_LEASE_TTL_MS,
        "heartbeat_interval_ms": WEB_LEASE_HEARTBEAT_INTERVAL_MS
    })
}

fn resolve_operation(
    state: &AppState,
    device_id: &str,
    target: Option<TargetKind>,
    artifact_id: Option<String>,
    dry_run: bool,
) -> Result<(FirmwareArtifact, TargetKind, bool, Value), HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get(device_id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    let artifact_id = artifact_id
        .or_else(|| device.selected_artifact_id.clone())
        .ok_or_else(|| {
            HttpError::bad_request("artifact_missing", "select an artifact before flashing")
        })?;
    let artifact = guard
        .artifacts
        .get(&artifact_id)
        .cloned()
        .ok_or_else(|| HttpError::not_found("artifact_not_found", "artifact is not loaded"))?;
    let target = target.unwrap_or_else(|| artifact.target.clone());
    if target != artifact.target {
        return Err(HttpError::conflict(
            "artifact_target_mismatch",
            "selected artifact target does not match requested operation target",
        ));
    }
    let evidence = target_evidence_locked(device, target.clone(), Some(&artifact))?;
    Ok((artifact, target, dry_run, evidence))
}

fn target_evidence(
    state: &AppState,
    device_id: &str,
    target: TargetKind,
    artifact: Option<&FirmwareArtifact>,
) -> Result<Value, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard
        .devices
        .get(device_id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    target_evidence_locked(device, target, artifact)
}

fn target_evidence_locked(
    device: &DeviceRecord,
    target: TargetKind,
    artifact: Option<&FirmwareArtifact>,
) -> Result<Value, HttpError> {
    let candidate = match target {
        TargetKind::DigitalEsp32s3 => device.digital_target.as_ref(),
        TargetKind::AnalogStm32g431 => device.analog_target.as_ref(),
        TargetKind::LanHttp => None,
        TargetKind::Mock => Some(&TargetCandidate {
            kind: TargetKind::Mock,
            display_name: device.display_name.clone(),
            port_path: None,
            probe_selector: None,
            lan_base_url: None,
            selector_source: None,
        }),
    };
    let Some(candidate) = candidate else {
        return Err(HttpError::conflict(
            "target_unavailable",
            "requested target is not available on this device",
        ));
    };
    Ok(json!({
        "device_id": device.id,
        "target": target,
        "display_name": candidate.display_name,
        "port_path": candidate.port_path,
        "probe_selector": candidate.probe_selector,
        "selector_source": candidate.selector_source,
        "artifact_id": artifact.map(|a| a.artifact_id.as_str()),
        "artifact_build_id": artifact.map(|a| a.build_id.as_str()),
    }))
}

fn ensure_real_operation_uses_cached_target(
    device: &DeviceRecord,
    target: &TargetKind,
) -> Result<(), HttpError> {
    let candidate = match target {
        TargetKind::DigitalEsp32s3 => device.digital_target.as_ref(),
        TargetKind::AnalogStm32g431 => device.analog_target.as_ref(),
        TargetKind::LanHttp | TargetKind::Mock => None,
    };
    let Some(candidate) = candidate else {
        return Err(HttpError::conflict(
            "target_unavailable",
            "requested target is not available on this device",
        ));
    };
    let expected_source = match target {
        TargetKind::DigitalEsp32s3 => DEFAULT_DIGITAL_USB_PORT_SELECTOR_SOURCE,
        TargetKind::AnalogStm32g431 => ".stm32-port",
        TargetKind::LanHttp | TargetKind::Mock => unreachable!(),
    };
    if candidate.selector_source.as_deref() != Some(expected_source) {
        return Err(HttpError::conflict(
            "target_selector_not_cached",
            format!(
                "real {target:?} operation requires the selected target to come from {expected_source}; run dry-run or ask the owner to approve an explicit selector update"
            ),
        ));
    }
    Ok(())
}

fn target_requires_usb_lease(target: &TargetKind) -> bool {
    matches!(target, TargetKind::DigitalEsp32s3)
}

#[derive(Debug)]
struct EspflashOperation {
    command: &'static str,
    file_path: String,
    flash_address: Option<u64>,
}

fn selected_espflash_operation(
    artifact: &FirmwareArtifact,
) -> Result<EspflashOperation, HttpError> {
    if let Some(file) = artifact.files.iter().find(|file| file.kind == "elf") {
        return Ok(EspflashOperation {
            command: "flash",
            file_path: file.path.clone(),
            flash_address: None,
        });
    }
    if let Some(file) = artifact.files.iter().find(|file| file.kind == "image") {
        let Some(flash_address) = file.flash_address else {
            return Err(HttpError::bad_request(
                "artifact_flash_address_missing",
                "image artifacts require flash_address for espflash write-bin",
            ));
        };
        return Ok(EspflashOperation {
            command: "write-bin",
            file_path: file.path.clone(),
            flash_address: Some(flash_address),
        });
    }
    Err(HttpError::bad_request(
        "artifact_flash_file_missing",
        "digital flash requires an artifact file with kind=elf or kind=image",
    ))
}

fn selected_analog_elf_file(artifact: &FirmwareArtifact) -> Result<String, HttpError> {
    artifact
        .files
        .iter()
        .find(|file| file.kind == "elf")
        .map(|file| file.path.clone())
        .ok_or_else(|| {
            HttpError::bad_request(
                "artifact_flash_file_missing",
                "analog flash requires an artifact file with kind=elf",
            )
        })
}

fn canonicalize_probe_rs_selector(selector: &str) -> String {
    let trimmed = selector.trim();
    let mut parts = trimmed.splitn(3, ':');
    let Some(vid) = parts.next() else {
        return trimmed.to_string();
    };
    let Some(pid_or_legacy) = parts.next() else {
        return trimmed.to_string();
    };
    let Some(serial) = parts.next() else {
        return trimmed.to_string();
    };
    let Some((pid, index)) = pid_or_legacy.split_once('-') else {
        return trimmed.to_string();
    };

    if is_hex4(vid)
        && is_hex4(pid)
        && !index.is_empty()
        && !serial.is_empty()
        && index.chars().all(|c| c.is_ascii_digit())
    {
        format!("{vid}:{pid}:{serial}")
    } else {
        trimmed.to_string()
    }
}

fn is_hex4(value: &str) -> bool {
    value.len() == 4 && value.chars().all(|c| c.is_ascii_hexdigit())
}

async fn run_espflash_digital(
    state: &AppState,
    device_id: &str,
    artifact: &FirmwareArtifact,
) -> Result<(), HttpError> {
    let (port_path, operation) = {
        let guard = state.inner.lock().expect("state lock");
        let device = guard
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        let target = device.digital_target.as_ref().ok_or_else(|| {
            HttpError::conflict("target_unavailable", "digital target is not available")
        })?;
        let port_path = target.port_path.clone().ok_or_else(|| {
            HttpError::conflict(
                "target_port_missing",
                "digital flash requires an approved ESP32-S3 USB port path",
            )
        })?;
        let operation = selected_espflash_operation(artifact)?;
        (port_path, operation)
    };
    let _exclusive = reserve_serial_exclusive(state, &port_path, "digital flash")?;

    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_log(
                device,
                "info",
                "flash",
                &format!("starting espflash for {}", artifact.artifact_id),
            );
            push_trace(
                device,
                "tx",
                json!({
                    "type": "flash",
                    "tool": "espflash",
                    "chip": "esp32s3",
                    "port": port_path,
                    "artifact_id": artifact.artifact_id,
                    "command": operation.command,
                    "file": operation.file_path,
                    "flash_address": operation.flash_address,
                }),
            );
        }
    }

    let espflash = env::var(ESPFLASH_ENV).unwrap_or_else(|_| DEFAULT_ESPFLASH.to_string());
    let mut command = Command::new(&espflash);
    command
        .arg(operation.command)
        .arg("--chip")
        .arg("esp32s3")
        .arg("--port")
        .arg(&port_path)
        .arg("--non-interactive");
    if let Some(flash_address) = operation.flash_address {
        command.arg(format!("0x{flash_address:x}"));
    }
    let output_result = command
        .arg(&operation.file_path)
        .stdin(Stdio::null())
        .output()
        .await;
    let output = output_result
        .map_err(|error| HttpError::retryable("espflash_launch_failed", error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_trace(
                device,
                "rx",
                json!({
                    "type": "flash_result",
                    "tool": espflash,
                    "status": output.status.code(),
                    "stdout_tail": tail_text(&stdout, 2000),
                    "stderr_tail": tail_text(&stderr, 2000),
                }),
            );
            push_log(
                device,
                if output.status.success() {
                    "info"
                } else {
                    "error"
                },
                "flash",
                if output.status.success() {
                    "espflash completed"
                } else {
                    "espflash failed"
                },
            );
            if output.status.success() {
                device.connection = ConnectionState::Disconnected;
            }
        }
    }

    if !output.status.success() {
        return Err(HttpError::retryable(
            "espflash_failed",
            format!(
                "espflash {} exited with {}",
                operation.command, output.status
            ),
        ));
    }
    Ok(())
}

async fn run_espflash_reset_digital(state: &AppState, device_id: &str) -> Result<(), HttpError> {
    let port_path = {
        let guard = state.inner.lock().expect("state lock");
        let device = guard
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        device
            .digital_target
            .as_ref()
            .and_then(|target| target.port_path.clone())
            .ok_or_else(|| {
                HttpError::conflict(
                    "target_port_missing",
                    "digital reset requires an approved ESP32-S3 USB port path",
                )
            })?
    };
    let _exclusive = reserve_serial_exclusive(state, &port_path, "digital reset")?;

    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_log(device, "info", "reset", "starting espflash reset");
            push_trace(
                device,
                "tx",
                json!({"type": "reset", "tool": "espflash", "chip": "esp32s3", "port": port_path}),
            );
        }
    }

    let espflash = env::var(ESPFLASH_ENV).unwrap_or_else(|_| DEFAULT_ESPFLASH.to_string());
    let output_result = Command::new(&espflash)
        .arg("reset")
        .arg("--chip")
        .arg("esp32s3")
        .arg("--port")
        .arg(&port_path)
        .arg("--non-interactive")
        .stdin(Stdio::null())
        .output()
        .await;
    let output = output_result
        .map_err(|error| HttpError::retryable("espflash_launch_failed", error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_trace(
                device,
                "rx",
                json!({
                    "type": "reset_result",
                    "tool": espflash,
                    "status": output.status.code(),
                    "stdout_tail": tail_text(&stdout, 2000),
                    "stderr_tail": tail_text(&stderr, 2000),
                }),
            );
            push_log(
                device,
                if output.status.success() {
                    "info"
                } else {
                    "error"
                },
                "reset",
                if output.status.success() {
                    "espflash reset completed"
                } else {
                    "espflash reset failed"
                },
            );
            if output.status.success() {
                device.connection = ConnectionState::Disconnected;
            }
        }
    }

    if !output.status.success() {
        return Err(HttpError::retryable(
            "espflash_failed",
            format!("espflash reset exited with {}", output.status),
        ));
    }
    Ok(())
}

async fn run_probe_rs_analog(
    state: &AppState,
    device_id: &str,
    artifact: &FirmwareArtifact,
) -> Result<(), HttpError> {
    let (probe_selector, elf_path) = {
        let guard = state.inner.lock().expect("state lock");
        let device = guard
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        let target = device.analog_target.as_ref().ok_or_else(|| {
            HttpError::conflict("target_unavailable", "analog target is not available")
        })?;
        let probe_selector = target.probe_selector.clone().ok_or_else(|| {
            HttpError::conflict(
                "target_probe_missing",
                "analog flash requires an approved STM32 probe selector",
            )
        })?;
        let elf_path = selected_analog_elf_file(artifact)?;
        (canonicalize_probe_rs_selector(&probe_selector), elf_path)
    };

    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_log(
                device,
                "info",
                "flash",
                &format!("starting probe-rs for {}", artifact.artifact_id),
            );
            push_trace(
                device,
                "tx",
                json!({
                    "type": "flash",
                    "tool": "probe-rs",
                    "chip": ANALOG_PROBE_CHIP,
                    "probe": probe_selector,
                    "artifact_id": artifact.artifact_id,
                    "command": "download",
                    "file": elf_path,
                }),
            );
        }
    }

    let probe_rs = env::var(PROBE_RS_ENV).unwrap_or_else(|_| DEFAULT_PROBE_RS.to_string());
    let output = Command::new(&probe_rs)
        .arg("download")
        .arg(&elf_path)
        .arg("--chip")
        .arg(ANALOG_PROBE_CHIP)
        .arg("--probe")
        .arg(&probe_selector)
        .arg("--non-interactive")
        .arg("--protocol")
        .arg(ANALOG_PROBE_PROTOCOL)
        .arg("--speed")
        .arg(ANALOG_PROBE_SPEED_KHZ.to_string())
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| HttpError::retryable("probe_rs_launch_failed", error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_trace(
                device,
                "rx",
                json!({
                    "type": "flash_result",
                    "tool": probe_rs,
                    "status": output.status.code(),
                    "stdout_tail": tail_text(&stdout, 2000),
                    "stderr_tail": tail_text(&stderr, 2000),
                }),
            );
            push_log(
                device,
                if output.status.success() {
                    "info"
                } else {
                    "error"
                },
                "flash",
                if output.status.success() {
                    "probe-rs completed"
                } else {
                    "probe-rs failed"
                },
            );
            if output.status.success() {
                device.connection = ConnectionState::Disconnected;
            }
        }
    }

    if !output.status.success() {
        return Err(HttpError::retryable(
            "probe_rs_failed",
            format!("probe-rs download exited with {}", output.status),
        ));
    }
    Ok(())
}

async fn run_probe_rs_reset_analog(state: &AppState, device_id: &str) -> Result<(), HttpError> {
    let probe_selector = {
        let guard = state.inner.lock().expect("state lock");
        let device = guard
            .devices
            .get(device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        let target = device.analog_target.as_ref().ok_or_else(|| {
            HttpError::conflict("target_unavailable", "analog target is not available")
        })?;
        target
            .probe_selector
            .as_deref()
            .map(canonicalize_probe_rs_selector)
            .ok_or_else(|| {
                HttpError::conflict(
                    "target_probe_missing",
                    "analog reset requires an approved STM32 probe selector",
                )
            })?
    };

    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_log(device, "info", "reset", "starting probe-rs reset");
            push_trace(
                device,
                "tx",
                json!({
                    "type": "reset",
                    "tool": "probe-rs",
                    "chip": ANALOG_PROBE_CHIP,
                    "probe": probe_selector,
                    "command": "reset",
                }),
            );
        }
    }

    let probe_rs = env::var(PROBE_RS_ENV).unwrap_or_else(|_| DEFAULT_PROBE_RS.to_string());
    let output = Command::new(&probe_rs)
        .arg("reset")
        .arg("--chip")
        .arg(ANALOG_PROBE_CHIP)
        .arg("--probe")
        .arg(&probe_selector)
        .arg("--non-interactive")
        .arg("--protocol")
        .arg(ANALOG_PROBE_PROTOCOL)
        .arg("--speed")
        .arg(ANALOG_PROBE_SPEED_KHZ.to_string())
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| HttpError::retryable("probe_rs_launch_failed", error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    {
        let mut guard = state.inner.lock().expect("state lock");
        if let Some(device) = guard.devices.get_mut(device_id) {
            push_trace(
                device,
                "rx",
                json!({
                    "type": "reset_result",
                    "tool": probe_rs,
                    "status": output.status.code(),
                    "stdout_tail": tail_text(&stdout, 2000),
                    "stderr_tail": tail_text(&stderr, 2000),
                }),
            );
            push_log(
                device,
                if output.status.success() {
                    "info"
                } else {
                    "error"
                },
                "reset",
                if output.status.success() {
                    "probe-rs reset completed"
                } else {
                    "probe-rs reset failed"
                },
            );
            if output.status.success() {
                device.connection = ConnectionState::Disconnected;
            }
        }
    }

    if !output.status.success() {
        return Err(HttpError::retryable(
            "probe_rs_failed",
            format!("probe-rs reset exited with {}", output.status),
        ));
    }
    Ok(())
}

fn tail_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars().rev().take(max_chars).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
}

fn read_manifest(path: &str) -> Result<Vec<FirmwareArtifact>, HttpError> {
    let text = fs::read_to_string(path).map_err(|error| {
        HttpError::retryable("artifact_read_failed", format!("{path}: {error}"))
    })?;
    let manifest: FirmwareManifest = serde_json::from_str(&text)
        .map_err(|error| HttpError::bad_request("artifact_parse_failed", error.to_string()))?;
    match manifest {
        FirmwareManifest::Artifact(artifact) => Ok(vec![*artifact]),
        FirmwareManifest::Catalog(catalog) => {
            if catalog.schema_version != "1" {
                return Err(HttpError::bad_request(
                    "artifact_catalog_version_unsupported",
                    format!(
                        "unsupported firmware catalog schema_version {}",
                        catalog.schema_version
                    ),
                ));
            }
            if catalog.artifacts.is_empty() {
                return Err(HttpError::bad_request(
                    "artifact_catalog_empty",
                    "firmware catalog contains no artifacts",
                ));
            }
            Ok(catalog.artifacts)
        }
    }
}

pub fn verify_artifact_files(artifact: &FirmwareArtifact) -> Result<(), HttpError> {
    for file in &artifact.files {
        let bytes = fs::read(&file.path).map_err(|error| {
            HttpError::retryable("artifact_read_failed", format!("{}: {error}", file.path))
        })?;
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != file.sha256 {
            return Err(HttpError::conflict(
                "artifact_sha256_mismatch",
                format!("{} expected {} got {}", file.path, file.sha256, actual),
            ));
        }
    }
    Ok(())
}

fn apply_artifact_match(device: &mut DeviceRecord, artifact: Option<&FirmwareArtifact>) {
    let Some(artifact) = artifact else {
        device.log_decode = LogDecodeState::default();
        return;
    };
    let firmware = device.identity.as_ref().and_then(|v| v.get("firmware"));
    let matched = firmware
        .and_then(|f| f.get("build_id"))
        .and_then(Value::as_str)
        == Some(artifact.build_id.as_str())
        && firmware
            .and_then(|f| f.get("build_profile"))
            .and_then(Value::as_str)
            == Some(artifact.build_profile.as_str())
        && identity_features_match(firmware, &artifact.features);
    device.log_decode = if matched {
        LogDecodeState {
            status: "verified".to_string(),
            reason: None,
            artifact_id: Some(artifact.artifact_id.clone()),
        }
    } else {
        LogDecodeState {
            status: "unverified".to_string(),
            reason: Some("device firmware identity does not match selected artifact".to_string()),
            artifact_id: Some(artifact.artifact_id.clone()),
        }
    };
}

fn status_cache_timestamp_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn maybe_update_device_status_cache(device: &mut DeviceRecord, value: &Value) -> bool {
    let Some(status) = value.get("status").cloned() else {
        return false;
    };
    device.status_cache = Some(status);
    if let Some(control) = value.get("control").cloned() {
        device.control_cache = Some(control);
    }
    device.status_meta_cache = Some(status_meta_from_status_bundle(value));
    device.status_cache_updated_at_ms = Some(status_cache_timestamp_ms());
    true
}

fn maybe_update_device_status_cache_from_status_payload(
    device: &mut DeviceRecord,
    status: &Value,
    control: Option<Value>,
) -> bool {
    if status.get("state_flags").is_none()
        || status.get("fault_flags").is_none()
        || status.get("v_local_mv").is_none()
        || status.get("i_local_ma").is_none()
    {
        return false;
    }
    device.status_cache = Some(status.clone());
    if let Some(control) = control {
        device.control_cache = Some(control);
    }
    device.status_meta_cache = None;
    device.status_cache_updated_at_ms = Some(status_cache_timestamp_ms());
    true
}

fn maybe_update_device_control_cache(device: &mut DeviceRecord, value: &Value) -> bool {
    if value.get("active_preset_id").is_none() {
        return false;
    }
    if value.get("preset").is_none()
        && !(value.get("mode").is_some() && value.get("output_enabled").is_some())
    {
        return false;
    }
    device.control_cache = Some(value.clone());
    true
}

fn cached_status_bundle(device: &DeviceRecord) -> Option<Value> {
    let status = device.status_cache.clone()?;
    let mut out = serde_json::Map::new();
    out.insert("status".to_string(), status);
    if let Some(control) = device.control_cache.clone() {
        out.insert("control".to_string(), control);
    }
    if let Some(meta) = device.status_meta_cache.as_ref().and_then(Value::as_object) {
        for key in [
            "link_up",
            "hello_seen",
            "analog_state",
            "fault_flags_decoded",
        ] {
            if let Some(value) = meta.get(key) {
                out.insert(key.to_string(), value.clone());
            }
        }
        out.insert("status_meta".to_string(), Value::Object(meta.clone()));
    }
    if let Some(updated_at_ms) = device.status_cache_updated_at_ms {
        out.insert("cache_updated_at_ms".to_string(), json!(updated_at_ms));
    }
    out.insert("from_monitor_cache".to_string(), json!(true));
    Some(Value::Object(out))
}

fn merge_cached_control_if_missing(device: &DeviceRecord, mut value: Value) -> Value {
    if value.get("control").is_some() {
        return value;
    }
    let Some(control) = device.control_cache.clone() else {
        return value;
    };
    if let Some(object) = value.as_object_mut() {
        object.insert("control".to_string(), control);
    }
    value
}

fn status_meta_from_status_bundle(value: &Value) -> Value {
    let mut meta = serde_json::Map::new();
    for key in [
        "link_up",
        "hello_seen",
        "analog_state",
        "fault_flags_decoded",
    ] {
        if let Some(field) = value.get(key).cloned() {
            meta.insert(key.to_string(), field);
        }
    }
    Value::Object(meta)
}

fn fresh_cached_status_data(state: &AppState, device_id: &str) -> Option<Value> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard.devices.get(device_id)?;
    let updated_at_ms = device.status_cache_updated_at_ms?;
    let age_ms = status_cache_timestamp_ms() - updated_at_ms;
    if age_ms > STATUS_CACHE_MAX_AGE_MS {
        return None;
    }
    cached_status_bundle(device).map(|mut value| {
        if let Some(object) = value.as_object_mut() {
            object.insert("cache_age_ms".to_string(), json!(age_ms.max(0)));
        }
        value
    })
}

fn identity_features_match(firmware: Option<&Value>, artifact_features: &[String]) -> bool {
    let Some(features) = firmware
        .and_then(|f| f.get("features"))
        .and_then(Value::as_array)
    else {
        return artifact_features.is_empty();
    };
    let mut identity = features
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut artifact = artifact_features.to_vec();
    identity.sort();
    artifact.sort();
    identity == artifact
}

fn scan_serial_targets(default_usb_port: Option<&str>) -> Vec<TargetCandidate> {
    let mut out = Vec::new();
    if let Some(port_path) = default_usb_port {
        out.push(TargetCandidate {
            kind: TargetKind::DigitalEsp32s3,
            display_name: format!("ESP32-S3 USB CDC ({port_path})"),
            port_path: Some(port_path.to_string()),
            probe_selector: None,
            lan_base_url: None,
            selector_source: Some(DEFAULT_DIGITAL_USB_PORT_SELECTOR_SOURCE.to_string()),
        });
        return out;
    }

    for port in list_digital_usb_port_candidates() {
        out.push(TargetCandidate {
            kind: TargetKind::DigitalEsp32s3,
            display_name: format!("ESP32-S3 USB CDC ({})", port.display_name),
            port_path: Some(port.port_path),
            probe_selector: None,
            lan_base_url: None,
            selector_source: Some("serialport scan".to_string()),
        });
    }
    out
}

fn device_digital_port(state: &AppState, device_id: &str) -> Option<String> {
    state
        .inner
        .lock()
        .expect("state lock")
        .devices
        .get(device_id)
        .and_then(|device| device.digital_target.as_ref())
        .and_then(|target| target.port_path.clone())
}

fn canonical_port_key(port_path: &str) -> String {
    port_path.trim().to_string()
}

fn sanitize_thread_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn next_request_id(op: &str) -> String {
    format!(
        "devd-{}-{}",
        op.replace('_', "-"),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

fn serial_request_expects_wifi_wait(op: &str, extra: Option<&Value>) -> bool {
    op == "set_wifi_config"
        && extra
            .and_then(|value| value.get("wait"))
            .and_then(Value::as_bool)
            == Some(true)
}

fn serial_protocol_timeout_ms(op: &str, extra: Option<&Value>) -> u64 {
    if serial_request_expects_wifi_wait(op, extra) {
        SERIAL_WIFI_WAIT_PROTOCOL_TIMEOUT_MS
    } else {
        SERIAL_PROTOCOL_TIMEOUT_MS
    }
}

fn serial_operation_wait_ms(op: &str, extra: Option<&Value>) -> u64 {
    if serial_request_expects_wifi_wait(op, extra) {
        SERIAL_WIFI_WAIT_OPERATION_WAIT_MS
    } else {
        SERIAL_OPERATION_WAIT_MS
    }
}

fn validate_port_not_leased_by_other_device(
    state: &AppState,
    device_id: &str,
    port_path: &str,
) -> Result<(), HttpError> {
    let port_key = canonical_port_key(port_path);
    let guard = state.inner.lock().expect("state lock");
    for lease in guard.leases.values() {
        if lease.expires_at <= Instant::now() || lease.device_id == device_id {
            continue;
        }
        let other_port = lease.port_path.clone().or_else(|| {
            guard
                .devices
                .get(&lease.device_id)
                .and_then(|device| device.digital_target.as_ref())
                .and_then(|target| target.port_path.clone())
        });
        let Some(other_port) = other_port.as_deref() else {
            continue;
        };
        if canonical_port_key(other_port) == port_key {
            return Err(HttpError::conflict(
                "device_port_in_use",
                format!(
                    "USB port {port_path} is currently leased by device {}",
                    lease.device_id
                ),
            ));
        }
    }
    Ok(())
}

fn update_device_identity(
    state: &AppState,
    device_id: &str,
    identity: Value,
) -> Result<(), HttpError> {
    let identity_device_id = identity
        .get("device_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut guard = state.inner.lock().expect("state lock");
    if let Some(identity_device_id) = identity_device_id.as_deref()
        && let Some((other_id, _)) = guard.devices.iter().find(|(id, device)| {
            id.as_str() != device_id
                && device
                    .identity
                    .as_ref()
                    .and_then(|value| value.get("device_id"))
                    .and_then(Value::as_str)
                    == Some(identity_device_id)
        })
    {
        return Err(HttpError::conflict(
            "device_identity_conflict",
            format!("USB identity {identity_device_id} is already bound to device {other_id}"),
        ));
    }

    let selected_artifact = guard
        .devices
        .get(device_id)
        .and_then(|device| device.selected_artifact_id.as_ref())
        .and_then(|id| guard.artifacts.get(id))
        .cloned();
    let device = guard
        .devices
        .get_mut(device_id)
        .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    device.identity = Some(identity);
    apply_artifact_match(device, selected_artifact.as_ref());
    Ok(())
}

fn update_device_identity_for_lease_probe(
    state: &AppState,
    device_id: &str,
    port_path: &str,
    identity: Value,
) -> Result<(), HttpError> {
    if let Err(error) = update_device_identity(state, device_id, identity) {
        stop_serial_owner(state, port_path);
        return Err(error);
    }
    Ok(())
}

fn device_id_for_port(state: &AppState, port_path: &str) -> Option<String> {
    let port_key = canonical_port_key(port_path);
    state
        .inner
        .lock()
        .expect("state lock")
        .devices
        .values()
        .find(|device| {
            device
                .digital_target
                .as_ref()
                .and_then(|target| target.port_path.as_deref())
                .is_some_and(|other| canonical_port_key(other) == port_key)
        })
        .map(|device| device.id.clone())
}

fn mock_serial_probe(
    state: &AppState,
    request_id: &str,
    op: &str,
    extra: Option<Value>,
) -> SerialProtocolProbe {
    #[cfg(test)]
    if let Some(mut queued) = state
        .mock_serial_responses
        .lock()
        .expect("mock serial responses lock")
        .pop_front()
    {
        if let Some(frame) = queued
            .frames
            .iter_mut()
            .find(|frame| frame.direction == "tx")
        {
            frame.frame = json!({"type": "request", "request_id": request_id, "op": op});
        } else {
            queued.frames.insert(
                0,
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": op}),
                },
            );
        }
        for frame in queued
            .frames
            .iter_mut()
            .filter(|frame| frame.direction == "rx")
        {
            if frame.frame.get("request_id").is_some() {
                frame.frame["request_id"] = json!(request_id);
            }
        }
        return queued;
    }
    #[cfg(not(test))]
    let _ = state;
    let data = match op {
        "get_identity" => mock_identity("mock-loadlynx-devd", "Mock LoadLynx devd device"),
        "get_status" => json!({
            "status": {"enable": false, "v_local_mv": 0, "i_local_ma": 0, "fault_flags": 0},
            "link_up": true,
            "hello_seen": true,
            "analog_state": "ready"
        }),
        "get_pd" | "set_pd_policy" => json!({
            "attached": false,
            "contract": null,
            "saved": extra.unwrap_or_else(|| json!({}))
        }),
        "set_output_enabled" => {
            json!({"enable": extra.and_then(|v| v.get("enable").cloned()).unwrap_or(json!(false))})
        }
        "get_wifi_credentials" => json!({
            "ssid": "LoadLynx-Test",
            "psk": "mock-loadlynx-psk",
            "source": "user"
        }),
        "get_wifi_status" | "set_wifi_config" | "clear_wifi_config" => json!({
            "ssid": extra.as_ref().and_then(|v| v.get("ssid")).and_then(Value::as_str).unwrap_or("LoadLynx-Test"),
            "source": if op == "clear_wifi_config" { "factory" } else { "user" },
            "state": "connected",
            "ip": "192.0.2.10",
            "last_error": null
        }),
        "get_control" | "set_control" | "apply_preset" => json!({
            "active_preset_id": extra.as_ref().and_then(|v| v.get("preset_id")).and_then(Value::as_u64).unwrap_or(1),
            "output_enabled": extra.as_ref().and_then(|v| v.get("output_enabled")).and_then(Value::as_bool).unwrap_or(false),
            "uv_latched": false,
            "preset": mock_preset(extra.as_ref().and_then(|v| v.get("preset_id")).and_then(Value::as_u64).unwrap_or(1) as u8)
        }),
        "get_presets" => json!({
            "presets": (1_u8..=5).map(mock_preset).collect::<Vec<_>>()
        }),
        "set_preset" => extra.unwrap_or_else(|| mock_preset(1)),
        "get_calibration_profile" => json!({
            "active": {"source": "factory-default", "fmt_version": 3, "hw_rev": 1},
            "current_ch1_points": [],
            "current_ch2_points": [],
            "v_local_points": [],
            "v_remote_points": []
        }),
        "calibration_apply" | "calibration_commit" | "calibration_reset" | "calibration_mode" => {
            json!({"ok": true})
        }
        "soft_reset" => json!({
            "accepted": true,
            "reason": extra.as_ref().and_then(|v| v.get("reason")).and_then(Value::as_str).unwrap_or("manual")
        }),
        "get_diagnostics" => json!({
            "schema_version": 1,
            "events": [],
            "redaction": {"psk": true, "password": true}
        }),
        _ => json!({}),
    };
    SerialProtocolProbe {
        frames: vec![
            SerialProtocolFrame {
                direction: "tx",
                frame: json!({"type": "request", "request_id": request_id, "op": op}),
            },
            SerialProtocolFrame {
                direction: "rx",
                frame: json!({"type": "response", "request_id": request_id, "ok": true, "data": data}),
            },
        ],
        non_protocol_bytes: 0,
        non_protocol_text: String::new(),
    }
}

fn mock_preset(preset_id: u8) -> Value {
    json!({
        "preset_id": preset_id.clamp(1, 5),
        "mode": "cc",
        "target_i_ma": 0,
        "target_v_mv": 12000,
        "target_p_mw": 0,
        "min_v_mv": 0,
        "max_i_ma_total": 10000,
        "max_p_mw": 120000
    })
}

fn record_serial_protocol_probe(
    state: &AppState,
    device_id: &str,
    port_path: &str,
    success_message: &str,
    probe: SerialProtocolProbe,
) {
    let mut guard = state.inner.lock().expect("state lock");
    let selected_artifact = guard
        .devices
        .get(device_id)
        .and_then(|device| device.selected_artifact_id.as_ref())
        .and_then(|id| guard.artifacts.get(id))
        .cloned();
    let Some(device) = guard.devices.get_mut(device_id) else {
        return;
    };
    match probe {
        probe if probe.frames.is_empty() && probe.non_protocol_bytes == 0 => {
            push_log(
                device,
                "info",
                "serial",
                &format!("serial probe opened {port_path}; no bytes before timeout"),
            );
            push_trace(
                device,
                "rx",
                json!({"type": "serial_probe", "port_path": port_path, "status": "timeout", "bytes": 0}),
            );
        }
        probe => {
            push_log(
                device,
                "info",
                "serial",
                &format!(
                    "{success_message}; opened {port_path}; decoded {} JSONL protocol frames ({} non-protocol bytes)",
                    probe.frames.len(),
                    probe.non_protocol_bytes
                ),
            );
            if probe.non_protocol_bytes != 0 {
                push_trace(
                    device,
                    "rx",
                    json!({
                        "type": "serial_probe",
                        "port_path": port_path,
                        "status": "non_protocol_bytes",
                        "bytes": probe.non_protocol_bytes,
                        "text": sanitize_trace_text(&probe.non_protocol_text)
                    }),
                );
            }
            for event in probe.frames {
                let request_id = event.frame.get("request_id").and_then(Value::as_str);
                if event.direction == "rx"
                    && event.frame.get("ok").and_then(Value::as_bool) == Some(true)
                    && request_id
                        .is_some_and(|id| serial_request_id_matches_op(id, "devd-get-identity"))
                    && let Some(data) = event.frame.get("data").cloned()
                {
                    device.identity = Some(data);
                    apply_artifact_match(device, selected_artifact.as_ref());
                } else if event.direction == "rx"
                    && event.frame.get("ok").and_then(Value::as_bool) == Some(true)
                    && request_id.is_some_and(|id| {
                        serial_request_id_matches_op(id, "devd-get-pd")
                            || serial_request_id_matches_op(id, "devd-set-pd-policy")
                    })
                    && let Some(data) = event.frame.get("data").cloned()
                {
                    device.usb_pd_cache = Some(data);
                }
                if event.direction == "rx"
                    && event.frame.get("ok").and_then(Value::as_bool) == Some(true)
                    && let Some(data) = event.frame.get("data")
                {
                    let _ = maybe_update_device_status_cache(device, data);
                    let _ = maybe_update_device_control_cache(device, data);
                } else if event.direction == "rx" {
                    let _ = maybe_update_device_status_cache(device, &event.frame);
                    let _ = maybe_update_device_control_cache(device, &event.frame);
                    let control = event
                        .frame
                        .get("active_preset_id")
                        .is_some()
                        .then(|| event.frame.clone());
                    let _ = maybe_update_device_status_cache_from_status_payload(
                        device,
                        &event.frame,
                        control,
                    );
                }
                push_trace(device, event.direction, event.frame);
            }
        }
    }
}

fn probe_has_satisfied_response(
    frames: &[SerialProtocolFrame],
    non_protocol_bytes: usize,
    non_protocol_text: &str,
    request_id: &str,
) -> bool {
    let probe = SerialProtocolProbe {
        frames: frames.to_vec(),
        non_protocol_bytes,
        non_protocol_text: non_protocol_text.to_string(),
    };
    serial_response_for_request(&probe, request_id).is_some()
        || infer_serial_response_from_fragments(&probe, request_id).is_some()
        || infer_serial_response_from_text(&probe, request_id).is_some()
}

fn finalize_status_output(device_id: &str, data: Value) -> Result<Value, HttpError> {
    let status = data.get("status").cloned().ok_or_else(|| {
        HttpError::retryable(
            "serial_response_invalid",
            "USB status response did not include status",
        )
    })?;
    let link_up = data.get("link_up").and_then(Value::as_bool).unwrap_or(true);
    let hello_seen = data
        .get("hello_seen")
        .and_then(Value::as_bool)
        .unwrap_or(link_up);
    let analog_state = data
        .get("analog_state")
        .and_then(Value::as_str)
        .unwrap_or(if link_up { "ready" } else { "offline" })
        .to_string();
    let mut output = data;
    if let Some(object) = output.as_object_mut() {
        object.insert("device_id".to_string(), json!(device_id));
        object.insert("status".to_string(), status);
        object
            .entry("link_up".to_string())
            .or_insert_with(|| json!(link_up));
        object
            .entry("hello_seen".to_string())
            .or_insert_with(|| json!(hello_seen));
        object
            .entry("analog_state".to_string())
            .or_insert_with(|| json!(analog_state));
        object
            .entry("fault_flags_decoded".to_string())
            .or_insert_with(|| json!([]));
    }
    Ok(output)
}

fn update_usb_status_cache(state: &AppState, device_id: &str, output: Value, source: &str) {
    let sampled_at_ms = now_unix_ms();
    let mut guard = state.inner.lock().expect("state lock");
    let Some(device) = guard.devices.get_mut(device_id) else {
        return;
    };
    device.usb_status_generation = device.usb_status_generation.saturating_add(1);
    device.usb_status_sampled_at_ms = Some(sampled_at_ms);
    device.usb_status_source = Some(source.to_string());
    device.usb_status_cache = Some(output);
}

fn attach_status_cache_metadata(
    output: &mut Value,
    generation: u64,
    sampled_at_ms: Option<i64>,
    source: Option<&str>,
) {
    let Some(object) = output.as_object_mut() else {
        return;
    };
    object.insert("status_sample_generation".to_string(), json!(generation));
    object.insert("status_cached".to_string(), json!(true));
    if let Some(sampled_at_ms) = sampled_at_ms {
        object.insert("status_sampled_at_ms".to_string(), json!(sampled_at_ms));
        if let Some(sampled_at_utc) = sampled_at_utc(sampled_at_ms) {
            object.insert("status_sampled_at_utc".to_string(), json!(sampled_at_utc));
        }
        object.insert(
            "status_sample_age_ms".to_string(),
            json!((now_unix_ms() - sampled_at_ms).max(0)),
        );
    }
    if let Some(source) = source {
        object.insert("status_sample_source".to_string(), json!(source));
    }
}

fn cached_usb_status_output(state: &AppState, device_id: &str, max_age_ms: i64) -> Option<Value> {
    let guard = state.inner.lock().expect("state lock");
    let device = guard.devices.get(device_id)?;
    let sampled_at_ms = device.usb_status_sampled_at_ms?;
    if max_age_ms >= 0 && now_unix_ms().saturating_sub(sampled_at_ms) > max_age_ms {
        return None;
    }
    let mut output = device.usb_status_cache.clone()?;
    attach_status_cache_metadata(
        &mut output,
        device.usb_status_generation,
        device.usb_status_sampled_at_ms,
        device.usb_status_source.as_deref(),
    );
    Some(output)
}

fn read_serial_jsonl_until(
    port: &mut dyn serialport::SerialPort,
    deadline: Instant,
    line_buf: &mut Vec<u8>,
    frames: &mut Vec<SerialProtocolFrame>,
    non_protocol_bytes: &mut usize,
    non_protocol_text: &mut String,
    wanted_request_id: Option<&str>,
) -> io::Result<bool> {
    let mut buf = [0_u8; 128];
    loop {
        if Instant::now() >= deadline {
            return Ok(false);
        }
        match port.read(&mut buf) {
            Ok(0) => return Ok(false),
            Ok(n) => {
                for &byte in &buf[..n] {
                    match byte {
                        b'\n' => {
                            let line = String::from_utf8_lossy(line_buf).trim().to_string();
                            line_buf.clear();
                            if line.is_empty() {
                                continue;
                            }
                            let parsed_frames = extract_serial_json_frames(&line);
                            if parsed_frames.is_empty() {
                                *non_protocol_bytes += line.len();
                                if non_protocol_text.len() < SERIAL_PROBE_MAX_BYTES {
                                    non_protocol_text.push_str(&line);
                                    non_protocol_text.push('\n');
                                }
                                if wanted_request_id.is_some_and(|request_id| {
                                    probe_has_satisfied_response(
                                        frames,
                                        *non_protocol_bytes,
                                        non_protocol_text,
                                        request_id,
                                    )
                                }) {
                                    return Ok(true);
                                }
                            } else {
                                for ExtractedSerialFrame {
                                    frame,
                                    non_protocol_bytes: skipped,
                                } in parsed_frames
                                {
                                    *non_protocol_bytes += skipped;
                                    let matched = wanted_request_id.is_some_and(|id| {
                                        frame
                                            .get("request_id")
                                            .and_then(Value::as_str)
                                            .is_some_and(|frame_id| frame_id == id)
                                    });
                                    frames.push(SerialProtocolFrame {
                                        direction: "rx",
                                        frame,
                                    });
                                    if matched {
                                        return Ok(true);
                                    }
                                    if wanted_request_id.is_some_and(|request_id| {
                                        probe_has_satisfied_response(
                                            frames,
                                            *non_protocol_bytes,
                                            non_protocol_text,
                                            request_id,
                                        )
                                    }) {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                        b'\r' => {}
                        byte => {
                            if line_buf.len() < SERIAL_PROBE_MAX_BYTES {
                                line_buf.push(byte);
                            } else {
                                *non_protocol_bytes += line_buf.len();
                                line_buf.clear();
                            }
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::TimedOut => continue,
            Err(error) => return Err(error),
        }
    }
}

fn write_serial_request(
    port: &mut dyn serialport::SerialPort,
    frames: &mut Vec<SerialProtocolFrame>,
    request_id: &str,
    op: &str,
    extra: Option<Value>,
) -> io::Result<()> {
    let mut frame = json!({
        "type": "request",
        "request_id": request_id,
        "op": op,
    });
    if let Some(extra) = extra
        && let (Some(frame), Some(extra)) = (frame.as_object_mut(), extra.as_object())
    {
        for (key, value) in extra {
            frame.insert(key.clone(), value.clone());
        }
    }
    let mut line = serde_json::to_vec(&frame)?;
    line.push(b'\n');
    port.write_all(&line)?;
    port.flush()?;
    frames.push(SerialProtocolFrame {
        direction: "tx",
        frame,
    });
    Ok(())
}

fn is_default_or_scanned_usb_source(state: &AppState, device_id: &str) -> bool {
    let guard = state.inner.lock().expect("state lock");
    guard
        .devices
        .get(device_id)
        .and_then(|device| device.digital_target.as_ref())
        .and_then(|target| target.selector_source.as_deref())
        .is_some_and(|source| {
            source == DEFAULT_DIGITAL_USB_PORT_SELECTOR_SOURCE || source == "serialport scan"
        })
}

fn serial_jsonl_request_on_port(
    port: &mut dyn serialport::SerialPort,
    request_id: &str,
    op: &str,
    extra: Option<Value>,
) -> io::Result<SerialProtocolProbe> {
    let protocol_timeout_ms = serial_protocol_timeout_ms(op, extra.as_ref());
    let mut frames = Vec::new();
    let mut line_buf = Vec::new();
    let mut non_protocol_bytes = 0usize;
    let mut non_protocol_text = String::new();

    let warmup_deadline = Instant::now() + Duration::from_millis(SERIAL_PROBE_TIMEOUT_MS);
    let _ = read_serial_jsonl_until(
        &mut *port,
        warmup_deadline,
        &mut line_buf,
        &mut frames,
        &mut non_protocol_bytes,
        &mut non_protocol_text,
        None,
    )?;
    line_buf.clear();

    write_serial_request(&mut *port, &mut frames, request_id, op, extra)?;
    line_buf.clear();
    let deadline = Instant::now() + Duration::from_millis(protocol_timeout_ms);
    while Instant::now() < deadline {
        let now = Instant::now();
        let slice_deadline =
            now + Duration::from_millis(50).min(deadline.saturating_duration_since(now));
        if read_serial_jsonl_until(
            &mut *port,
            slice_deadline,
            &mut line_buf,
            &mut frames,
            &mut non_protocol_bytes,
            &mut non_protocol_text,
            Some(request_id),
        )? {
            break;
        }
        if probe_has_satisfied_response(&frames, non_protocol_bytes, &non_protocol_text, request_id)
        {
            break;
        }
    }

    Ok(SerialProtocolProbe {
        frames,
        non_protocol_bytes,
        non_protocol_text,
    })
}

fn serial_status_poll_on_port(
    port: &mut dyn serialport::SerialPort,
    request_id: &str,
) -> io::Result<SerialProtocolProbe> {
    let mut frames = Vec::new();
    let mut line_buf = Vec::new();
    let mut non_protocol_bytes = 0usize;
    let mut non_protocol_text = String::new();

    let drain_deadline = Instant::now() + Duration::from_millis(10);
    let _ = read_serial_jsonl_until(
        &mut *port,
        drain_deadline,
        &mut line_buf,
        &mut frames,
        &mut non_protocol_bytes,
        &mut non_protocol_text,
        None,
    )?;
    line_buf.clear();

    write_serial_request(&mut *port, &mut frames, request_id, "get_status", None)?;
    let deadline = Instant::now() + Duration::from_millis(SERIAL_STATUS_REQUEST_TIMEOUT_MS);
    let _ = read_serial_jsonl_until(
        &mut *port,
        deadline,
        &mut line_buf,
        &mut frames,
        &mut non_protocol_bytes,
        &mut non_protocol_text,
        Some(request_id),
    )?;

    Ok(SerialProtocolProbe {
        frames,
        non_protocol_bytes,
        non_protocol_text,
    })
}

fn serial_worker_loop(
    state: AppState,
    port_path: String,
    owner_id: u64,
    rx: std_mpsc::Receiver<SerialWorkerCommand>,
) {
    let port_key = canonical_port_key(&port_path);
    let mut port = None;
    let mut last_idle_open_attempt = None;
    let mut last_status_poll_attempt = None;
    let mut idle_line_buf = Vec::new();
    let mut idle_frames = Vec::new();
    let mut idle_non_protocol_text = String::new();
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(command) => {
                let request = command.request.clone();
                if port.is_none() {
                    match open_serial_port(&port_path) {
                        Ok(opened) => port = Some(opened),
                        Err(error) => {
                            let _ = command.reply.send(Err(SerialWorkerError {
                                code: "serial_open_failed",
                                message: format!("{port_path}: {error}"),
                                retryable: true,
                            }));
                            continue;
                        }
                    }
                }
                let result = match serial_jsonl_request_on_port(
                    port.as_mut().expect("serial port opened").as_mut(),
                    &request.request_id,
                    &request.op,
                    request.extra,
                ) {
                    Ok(probe) => {
                        if serial_response_for_request(&probe, &request.request_id).is_some()
                            || infer_serial_response_from_fragments(&probe, &request.request_id)
                                .is_some()
                            || infer_serial_response_from_text(&probe, &request.request_id)
                                .is_some()
                        {
                            Ok(probe)
                        } else {
                            let code = if serial_probe_has_mismatched_response(
                                &probe,
                                &request.request_id,
                            ) {
                                "serial_response_mismatch"
                            } else {
                                "serial_response_timeout"
                            };
                            record_serial_protocol_probe(
                                &state,
                                &request.device_id,
                                &request.port_path,
                                "USB request completed without matching response",
                                probe,
                            );
                            Err(SerialWorkerError {
                                code,
                                message: format!(
                                    "USB request {} did not receive a matching response",
                                    request.request_id
                                ),
                                retryable: true,
                            })
                        }
                    }
                    Err(error) => {
                        port = None;
                        Err(SerialWorkerError {
                            code: "serial_request_failed",
                            message: error.to_string(),
                            retryable: true,
                        })
                    }
                };
                last_status_poll_attempt = None;
                let _ = command.reply.send(result);
            }
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                if port.is_none()
                    && !port_path.starts_with("mock://")
                    && last_idle_open_attempt.is_none_or(|attempt: Instant| {
                        attempt.elapsed() >= Duration::from_millis(500)
                    })
                {
                    last_idle_open_attempt = Some(Instant::now());
                    if let Ok(opened) = open_serial_port(&port_path) {
                        port = Some(opened);
                    }
                }
                let Some(open_port) = port.as_mut() else {
                    continue;
                };
                idle_frames.clear();
                let mut idle_non_protocol_bytes = 0usize;
                idle_non_protocol_text.clear();
                let deadline = Instant::now() + Duration::from_millis(25);
                if read_serial_jsonl_until(
                    open_port.as_mut(),
                    deadline,
                    &mut idle_line_buf,
                    &mut idle_frames,
                    &mut idle_non_protocol_bytes,
                    &mut idle_non_protocol_text,
                    None,
                )
                .is_ok()
                    && (!idle_frames.is_empty() || idle_non_protocol_bytes != 0)
                {
                    record_serial_protocol_probe(
                        &state,
                        device_id_for_port(&state, &port_path)
                            .as_deref()
                            .unwrap_or("unknown-device"),
                        &port_path,
                        "USB monitor frame received",
                        SerialProtocolProbe {
                            frames: std::mem::take(&mut idle_frames),
                            non_protocol_bytes: idle_non_protocol_bytes,
                            non_protocol_text: idle_non_protocol_text.clone(),
                        },
                    );
                }
                if !has_active_lease_for_port(&state, &port_path) {
                    continue;
                }
                if last_status_poll_attempt.is_some_and(|attempt: Instant| {
                    attempt.elapsed() < Duration::from_millis(SERIAL_STATUS_POLL_INTERVAL_MS)
                }) {
                    continue;
                }
                let Some(device_id) = device_id_for_port(&state, &port_path) else {
                    continue;
                };
                let request_id = next_request_id("status-cache");
                match serial_status_poll_on_port(open_port.as_mut(), &request_id) {
                    Ok(probe) => {
                        let response = serial_response_for_request(&probe, &request_id)
                            .or_else(|| infer_serial_response_from_fragments(&probe, &request_id))
                            .or_else(|| infer_serial_response_from_text(&probe, &request_id));
                        if let Ok(data) = status_data_from_serial_response(response)
                            && let Ok(output) = finalize_status_output(&device_id, data)
                        {
                            update_usb_status_cache(
                                &state,
                                &device_id,
                                output,
                                "serial_owner_status_poll",
                            );
                        }
                    }
                    Err(_) => {
                        port = None;
                    }
                }
                last_status_poll_attempt = Some(Instant::now());
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let mut registry = state.serial.lock().expect("serial registry lock");
    if registry
        .owners
        .get(&port_key)
        .is_some_and(|owner| owner.id == owner_id)
    {
        registry.owners.remove(&port_key);
    }
}

fn open_serial_port(port_path: &str) -> io::Result<Box<dyn serialport::SerialPort>> {
    Ok(serialport::new(port_path, SERIAL_PROBE_BAUD)
        .timeout(Duration::from_millis(SERIAL_PROBE_TIMEOUT_MS))
        .open()?)
}

pub fn default_digital_usb_port_path(repo_root: &std::path::Path) -> PathBuf {
    repo_root.join(DEFAULT_DIGITAL_USB_PORT_FILE)
}

pub fn default_analog_probe_path(repo_root: &std::path::Path) -> PathBuf {
    repo_root.join(DEFAULT_ANALOG_PROBE_FILE)
}

pub fn read_default_digital_usb_port(repo_root: &std::path::Path) -> Option<String> {
    fs::read_to_string(default_digital_usb_port_path(repo_root))
        .ok()?
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.contains('='))
        .map(ToOwned::to_owned)
}

pub fn read_default_analog_probe_selector(repo_root: &std::path::Path) -> Option<String> {
    fs::read_to_string(default_analog_probe_path(repo_root))
        .ok()?
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.contains('='))
        .map(ToOwned::to_owned)
}

pub fn write_default_digital_usb_port(
    repo_root: &std::path::Path,
    port_path: &str,
) -> io::Result<()> {
    let port_path = port_path.trim();
    if port_path.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "USB port path must not be empty",
        ));
    }
    fs::write(
        default_digital_usb_port_path(repo_root),
        format!("{port_path}\n"),
    )
}

pub fn list_digital_usb_port_candidates() -> Vec<DigitalUsbPortCandidate> {
    let Ok(ports) = serialport::available_ports() else {
        return Vec::new();
    };
    ports
        .into_iter()
        .filter(is_espflash_default_port_candidate)
        .map(|port| DigitalUsbPortCandidate {
            port_path: port.port_name.clone(),
            display_name: espflash_port_display_name(&port),
            recognized: is_espflash_known_port(&port),
        })
        .collect()
}

fn is_espflash_default_port_candidate(port: &serialport::SerialPortInfo) -> bool {
    if port.port_name.starts_with("/dev/tty.") {
        return false;
    }

    matches!(&port.port_type, serialport::SerialPortType::UsbPort(_))
}

fn is_espflash_known_port(port: &serialport::SerialPortInfo) -> bool {
    let serialport::SerialPortType::UsbPort(info) = &port.port_type else {
        return false;
    };
    matches!(
        (info.vid, info.pid),
        (0x10c4, 0xea60) | (0x1a86, 0x7523) | (0x303a, 0x1001)
    )
}

fn espflash_port_display_name(port: &serialport::SerialPortInfo) -> String {
    match &port.port_type {
        serialport::SerialPortType::UsbPort(info) => match &info.product {
            Some(product) => format!("{} - {product}", port.port_name),
            None => port.port_name.clone(),
        },
        _ => port.port_name.clone(),
    }
}

fn default_analog_probe_candidate(selector: &str) -> TargetCandidate {
    TargetCandidate {
        kind: TargetKind::AnalogStm32g431,
        display_name: format!("STM32G431 probe ({selector})"),
        port_path: None,
        probe_selector: Some(selector.to_string()),
        lan_base_url: None,
        selector_source: Some(DEFAULT_ANALOG_PROBE_SELECTOR_SOURCE.to_string()),
    }
}

fn stable_candidate_id(candidate: &TargetCandidate) -> String {
    let mut hash = Sha256::new();
    hash.update(format!("{:?}", candidate.kind));
    hash.update(candidate.port_path.as_deref().unwrap_or(""));
    hash.update(candidate.probe_selector.as_deref().unwrap_or(""));
    hash.update(candidate.lan_base_url.as_deref().unwrap_or(""));
    let digest = format!("{:x}", hash.finalize());
    match candidate.kind {
        TargetKind::DigitalEsp32s3 => format!("digital-{}", &digest[..12]),
        TargetKind::AnalogStm32g431 => format!("analog-{}", &digest[..12]),
        TargetKind::LanHttp => format!("lan-{}", &digest[..12]),
        TargetKind::Mock => format!("mock-{}", &digest[..12]),
    }
}

fn seed_mock_device(state: &AppState) {
    let mut device = DeviceRecord {
        id: "mock-loadlynx-devd".to_string(),
        display_name: "Mock LoadLynx devd device".to_string(),
        connection: ConnectionState::Disconnected,
        digital_target: Some(TargetCandidate {
            kind: TargetKind::DigitalEsp32s3,
            display_name: "Mock ESP32-S3".to_string(),
            port_path: Some("mock://esp32s3".to_string()),
            probe_selector: None,
            lan_base_url: None,
            selector_source: Some("mock".to_string()),
        }),
        analog_target: Some(TargetCandidate {
            kind: TargetKind::AnalogStm32g431,
            display_name: "Mock STM32G431".to_string(),
            port_path: None,
            probe_selector: Some("mock-probe".to_string()),
            lan_base_url: None,
            selector_source: Some("mock".to_string()),
        }),
        lan_endpoint: Some("mock://loadlynx-devd".to_string()),
        identity: Some(mock_identity(
            "mock-loadlynx-devd",
            "Mock LoadLynx devd device",
        )),
        usb_pd_cache: None,
        status_cache: None,
        control_cache: None,
        status_meta_cache: None,
        status_cache_updated_at_ms: None,
        usb_status_cache: None,
        usb_status_generation: 0,
        usb_status_sampled_at_ms: None,
        usb_status_source: None,
        selected_artifact_id: None,
        log_decode: LogDecodeState::default(),
        logs: VecDeque::new(),
        trace: VecDeque::new(),
    };
    push_log(&mut device, "info", "devd", "mock device seeded");
    state
        .inner
        .lock()
        .expect("state lock")
        .devices
        .insert(device.id.clone(), device);
}

fn mock_identity(id: &str, name: &str) -> Value {
    json!({
        "device_id": id,
        "hostname": "loadlynx-devd-mock.local",
        "short_id": "devd01",
        "digital_fw_version": "digital 0.1.0 (mock)",
        "analog_fw_version": "analog 0.1.0 (mock)",
        "protocol_version": 1,
        "uptime_ms": 0,
        "network": {"ip": "127.0.0.1", "mac": "00:00:00:00:00:00", "hostname": "loadlynx-devd-mock.local"},
        "firmware": {
            "name": name,
            "build_id": "mock-build",
            "build_profile": "debug",
            "features": ["net_http", "usb_cdc_bridge"],
            "protocol": "loadlynx.cdc.v1",
            "defmt": {"enabled": true, "encoding": "defmt"}
        },
        "capabilities": {
            "cc_supported": true,
            "cv_supported": true,
            "cp_supported": true,
            "presets_supported": true,
            "preset_count": 5,
            "api_version": "2.0.0",
            "devd": true,
            "usb_cdc_bridge": true,
            "mdns": true,
            "dns_sd": true
        }
    })
}

fn push_log(device: &mut DeviceRecord, level: &str, target: &str, message: &str) {
    push_bounded(
        &mut device.logs,
        SessionLog {
            id: next_id(),
            timestamp: now(),
            level: level.to_string(),
            target: target.to_string(),
            message: message.to_string(),
        },
        LOG_LIMIT,
    );
}

fn push_trace(device: &mut DeviceRecord, direction: &str, payload: Value) {
    let redacted = redact_sensitive_frame(&payload);
    push_bounded(
        &mut device.trace,
        SessionTrace {
            id: next_id(),
            timestamp: now(),
            direction: direction.to_string(),
            summary: summarize_frame(&redacted),
            payload: redacted,
        },
        TRACE_LIMIT,
    );
}

pub fn redact_sensitive_frame(frame: &Value) -> Value {
    match frame {
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
                        (key.clone(), redact_sensitive_frame(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_sensitive_frame).collect()),
        _ => frame.clone(),
    }
}

fn summarize_frame(frame: &Value) -> String {
    frame
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("frame")
        .to_string()
}

fn emit(state: &AppState, device_id: Option<String>, kind: &str, message: &str, payload: Value) {
    let event = DevdEvent {
        id: next_id(),
        timestamp: now(),
        device_id,
        kind: kind.to_string(),
        message: message.to_string(),
        payload,
    };
    {
        let mut guard = state.inner.lock().expect("state lock");
        push_bounded(&mut guard.events, event.clone(), EVENT_LIMIT);
    }
    let _ = state.events.send(event);
}

fn push_bounded<T>(queue: &mut VecDeque<T>, item: T, limit: usize) {
    queue.push_back(item);
    while queue.len() > limit {
        queue.pop_front();
    }
}

fn tail<T: Clone>(queue: &VecDeque<T>, limit: usize) -> Vec<T> {
    let len = queue.len();
    queue
        .iter()
        .skip(len.saturating_sub(limit))
        .cloned()
        .collect()
}

fn next_id() -> String {
    format!(
        "devd-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

fn now_unix_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn sampled_at_utc(sampled_at_ms: i64) -> Option<String> {
    chrono::DateTime::<Utc>::from_timestamp_millis(sampled_at_ms).map(|dt| dt.to_rfc3339())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

impl HttpError {
    fn bad_request(code: &str, message: impl Into<String>) -> Self {
        Self(error(code, message, false, None), StatusCode::BAD_REQUEST)
    }

    fn conflict(code: &str, message: impl Into<String>) -> Self {
        Self(error(code, message, false, None), StatusCode::CONFLICT)
    }

    fn retryable(code: &str, message: impl Into<String>) -> Self {
        Self(
            error(code, message, true, None),
            StatusCode::SERVICE_UNAVAILABLE,
        )
    }

    fn not_found(code: &str, message: impl Into<String>) -> Self {
        Self(error(code, message, false, None), StatusCode::NOT_FOUND)
    }
}

fn error(
    code: &str,
    message: impl Into<String>,
    retryable: bool,
    details: Option<Value>,
) -> ApiError {
    ApiError {
        code: code.to_string(),
        message: message.into(),
        retryable,
        details,
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let body = Json(ApiErrorEnvelope { error: self.0 });
        (self.1, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};

    fn test_artifact(artifact_id: &str, target: TargetKind) -> FirmwareArtifact {
        FirmwareArtifact {
            artifact_id: artifact_id.into(),
            name: artifact_id.into(),
            target,
            package_version: "0.1.0".into(),
            git_sha: "abc".into(),
            build_id: "b".into(),
            build_profile: "release".into(),
            features: vec![],
            protocol: "loadlynx.cdc.v1".into(),
            defmt: DefmtMetadata {
                enabled: true,
                encoding: "defmt".into(),
                elf_sha256: None,
                table_sha256: None,
            },
            files: vec![],
        }
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn ipc_serve_zero_idle_timeout_stays_alive_until_aborted() {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = dir.path().join("loadlynx-devd.sock");
        let endpoint = endpoint.to_string_lossy().to_string();
        let task = tokio::spawn(serve_ipc(IpcConfig::new(endpoint.clone(), Duration::ZERO)));

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !task.is_finished(),
            "idle_timeout=0 must disable idle shutdown"
        );

        let response = ipc_request(
            &endpoint,
            IpcRequest {
                op: "health".to_string(),
                params: json!({}),
            },
        )
        .await
        .unwrap();
        assert!(response.ok);
        assert!(
            !task.is_finished(),
            "server must remain alive after IPC request"
        );

        task.abort();
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn ipc_serve_nonzero_idle_timeout_exits_when_idle() {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = dir.path().join("loadlynx-devd.sock");
        let endpoint = endpoint.to_string_lossy().to_string();
        let task = tokio::spawn(serve_ipc(IpcConfig::new(
            endpoint,
            Duration::from_millis(25),
        )));

        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("server should exit after nonzero idle timeout")
            .unwrap()
            .unwrap();
    }

    #[test]
    fn expands_compact_usb_calibration_profile() {
        let expanded = expand_compact_calibration_profile(json!({
            "compact": "cal_profile_v1",
            "a": ["user-calibrated", 3, 1],
            "c1": [[0, 0, 0], [25000, 4095, 5000]],
            "c2": [[1, 2, 3]],
            "vl": [[0, 0]],
            "vr": [[100, 120]]
        }))
        .unwrap();

        assert_eq!(expanded["active"]["source"], "user-calibrated");
        assert_eq!(expanded["active"]["fmt_version"], 3);
        assert_eq!(expanded["current_ch1_points"][1]["raw_dac_code"], 4095);
        assert_eq!(expanded["current_ch2_points"][0]["meas_ma"], 3);
        assert_eq!(expanded["v_remote_points"][0]["meas_mv"], 120);
    }

    #[test]
    fn rejects_malformed_compact_usb_calibration_profile() {
        let err = expand_compact_calibration_profile(json!({
            "compact": "cal_profile_v1",
            "a": ["factory-default", 3, 1],
            "c1": [[0, 0]],
            "c2": [],
            "vl": [],
            "vr": []
        }))
        .unwrap_err();

        assert_eq!(err.0.code, "serial_response_invalid");
    }

    #[test]
    fn stable_candidate_id_is_deterministic() {
        let candidate = TargetCandidate {
            kind: TargetKind::DigitalEsp32s3,
            display_name: "A".into(),
            port_path: Some("/dev/cu.usbmodem1".into()),
            probe_selector: None,
            lan_base_url: None,
            selector_source: None,
        };
        assert_eq!(
            stable_candidate_id(&candidate),
            stable_candidate_id(&candidate)
        );
    }

    #[test]
    fn espflash_port_helpers_match_default_candidates() {
        assert!(is_espflash_default_port_candidate(&SerialPortInfo {
            port_name: "/dev/cu.usbmodem101".into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x303a,
                pid: 0x1001,
                serial_number: Some("abc".into()),
                manufacturer: None,
                product: Some("USB JTAG/serial debug unit".into()),
            }),
        }));
        assert!(is_espflash_known_port(&SerialPortInfo {
            port_name: "/dev/cu.usbmodem101".into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x303a,
                pid: 0x1001,
                serial_number: Some("abc".into()),
                manufacturer: None,
                product: None,
            }),
        }));
        assert!(!is_espflash_default_port_candidate(&SerialPortInfo {
            port_name: "/dev/tty.usbmodem101".into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x303a,
                pid: 0x1001,
                serial_number: Some("abc".into()),
                manufacturer: None,
                product: None,
            }),
        }));
        assert!(!is_espflash_default_port_candidate(&SerialPortInfo {
            port_name: "/dev/cu.usbmodem101".into(),
            port_type: SerialPortType::Unknown,
        }));
        assert!(
            espflash_port_display_name(&SerialPortInfo {
                port_name: "/dev/cu.usbmodem101".into(),
                port_type: SerialPortType::UsbPort(UsbPortInfo {
                    vid: 0x303a,
                    pid: 0x1001,
                    serial_number: Some("abc".into()),
                    manufacturer: None,
                    product: Some("USB JTAG/serial debug unit".into()),
                }),
            })
            .contains("USB JTAG/serial debug unit")
        );
        assert!(!is_espflash_known_port(&SerialPortInfo {
            port_name: "/dev/cu.usbmodem101".into(),
            port_type: SerialPortType::Unknown,
        }));
    }

    #[test]
    fn loopback_dev_origin_allows_only_local_http_ports() {
        assert!(is_loopback_dev_origin(&HeaderValue::from_static(
            "http://localhost:47821"
        )));
        assert!(is_loopback_dev_origin(&HeaderValue::from_static(
            "http://127.0.0.1:47821"
        )));
        assert!(is_loopback_dev_origin(&HeaderValue::from_static(
            "http://[::1]:47821"
        )));
        assert!(!is_loopback_dev_origin(&HeaderValue::from_static(
            "https://127.0.0.1:47821"
        )));
        assert!(!is_loopback_dev_origin(&HeaderValue::from_static(
            "http://example.com:47821"
        )));
    }

    #[test]
    fn redacts_wifi_psk() {
        let frame = json!({"type": "wifi_config", "op": "set", "ssid": "lab", "psk": "secret"});
        let redacted = redact_sensitive_frame(&frame);
        assert_eq!(redacted["psk"], "<redacted>");
        assert_eq!(redacted["ssid"], "lab");
    }

    #[test]
    fn artifact_hash_verification_detects_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("fw.bin");
        fs::write(&file_path, b"abc").unwrap();
        let mut artifact = test_artifact("a", TargetKind::DigitalEsp32s3);
        artifact.build_profile = "debug".into();
        artifact.protocol = "p".into();
        artifact.files = vec![ArtifactFile {
            kind: "image".into(),
            path: file_path.display().to_string(),
            sha256: "bad".into(),
            size: 3,
            flash_address: None,
        }];
        assert!(verify_artifact_files(&artifact).is_err());
    }

    #[test]
    fn espflash_operation_prefers_elf_flash() {
        let mut artifact = test_artifact("a", TargetKind::DigitalEsp32s3);
        artifact.files = vec![
            ArtifactFile {
                kind: "image".into(),
                path: "firmware.bin".into(),
                sha256: "unused".into(),
                size: 1,
                flash_address: Some(0x10000),
            },
            ArtifactFile {
                kind: "elf".into(),
                path: "digital".into(),
                sha256: "unused".into(),
                size: 1,
                flash_address: None,
            },
        ];

        let op = selected_espflash_operation(&artifact).unwrap();
        assert_eq!(op.command, "flash");
        assert_eq!(op.file_path, "digital");
        assert_eq!(op.flash_address, None);
    }

    #[test]
    fn espflash_image_operation_requires_flash_address() {
        let mut artifact = test_artifact("a", TargetKind::DigitalEsp32s3);
        artifact.files = vec![ArtifactFile {
            kind: "image".into(),
            path: "firmware.bin".into(),
            sha256: "unused".into(),
            size: 1,
            flash_address: None,
        }];

        let err = selected_espflash_operation(&artifact).unwrap_err();
        assert_eq!(err.0.code, "artifact_flash_address_missing");

        artifact.files[0].flash_address = Some(0x10000);
        let op = selected_espflash_operation(&artifact).unwrap();
        assert_eq!(op.command, "write-bin");
        assert_eq!(op.file_path, "firmware.bin");
        assert_eq!(op.flash_address, Some(0x10000));
    }

    #[test]
    fn analog_flash_uses_selected_elf_file_only() {
        let mut artifact = test_artifact("analog", TargetKind::AnalogStm32g431);
        artifact.files = vec![
            ArtifactFile {
                kind: "image".into(),
                path: "analog.bin".into(),
                sha256: "unused".into(),
                size: 1,
                flash_address: Some(0x08000000),
            },
            ArtifactFile {
                kind: "elf".into(),
                path: "analog.elf".into(),
                sha256: "unused".into(),
                size: 1,
                flash_address: None,
            },
        ];

        assert_eq!(selected_analog_elf_file(&artifact).unwrap(), "analog.elf");

        artifact.files.retain(|file| file.kind != "elf");
        let err = selected_analog_elf_file(&artifact).unwrap_err();
        assert_eq!(err.0.code, "artifact_flash_file_missing");
    }

    #[test]
    fn probe_rs_selector_canonicalizes_legacy_index() {
        assert_eq!(
            canonicalize_probe_rs_selector("0483:3748-2:SERIAL"),
            "0483:3748:SERIAL"
        );
        assert_eq!(
            canonicalize_probe_rs_selector("0483:3748:SERIAL"),
            "0483:3748:SERIAL"
        );
    }

    #[test]
    fn manifest_reader_accepts_single_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact.json");
        let artifact = test_artifact("digital", TargetKind::DigitalEsp32s3);
        fs::write(&path, serde_json::to_string(&artifact).unwrap()).unwrap();

        let artifacts = read_manifest(path.to_str().unwrap()).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_id, "digital");
    }

    #[test]
    fn manifest_reader_accepts_firmware_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        let catalog = FirmwareCatalog {
            schema_version: "1".into(),
            artifacts: vec![
                test_artifact("digital", TargetKind::DigitalEsp32s3),
                test_artifact("analog", TargetKind::AnalogStm32g431),
            ],
        };
        fs::write(&path, serde_json::to_string(&catalog).unwrap()).unwrap();

        let artifacts = read_manifest(path.to_str().unwrap()).unwrap();
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts[0].artifact_id, "digital");
        assert_eq!(artifacts[1].artifact_id, "analog");
    }

    #[tokio::test]
    async fn select_artifact_requires_id_for_multi_artifact_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        let catalog = FirmwareCatalog {
            schema_version: "1".into(),
            artifacts: vec![
                test_artifact("digital", TargetKind::DigitalEsp32s3),
                test_artifact("analog", TargetKind::AnalogStm32g431),
            ],
        };
        fs::write(&path, serde_json::to_string(&catalog).unwrap()).unwrap();

        let err = select_artifact(
            State(AppState::new(PathBuf::from("."))),
            Path("mock-loadlynx-devd".to_string()),
            Json(ArtifactSelectRequest {
                manifest_path: Some(path.display().to_string()),
                artifact_id: None,
                artifact: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "artifact_id_required");
    }

    #[tokio::test]
    async fn select_artifact_can_pick_from_firmware_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        let catalog = FirmwareCatalog {
            schema_version: "1".into(),
            artifacts: vec![
                test_artifact("digital", TargetKind::DigitalEsp32s3),
                test_artifact("analog", TargetKind::AnalogStm32g431),
            ],
        };
        fs::write(&path, serde_json::to_string(&catalog).unwrap()).unwrap();

        let state = AppState::new(PathBuf::from("."));
        let Json(body) = select_artifact(
            State(state.clone()),
            Path("mock-loadlynx-devd".to_string()),
            Json(ArtifactSelectRequest {
                manifest_path: Some(path.display().to_string()),
                artifact_id: Some("analog".to_string()),
                artifact: None,
            }),
        )
        .await
        .unwrap();

        assert_eq!(body["artifact"]["artifact_id"], "analog");
        let guard = state.inner.lock().expect("state lock");
        assert!(guard.artifacts.contains_key("digital"));
        assert!(guard.artifacts.contains_key("analog"));
        assert_eq!(
            guard
                .devices
                .get("mock-loadlynx-devd")
                .unwrap()
                .selected_artifact_id
                .as_deref(),
            Some("analog")
        );
    }

    #[test]
    fn artifact_match_requires_build_profile_and_features() {
        let mut device = DeviceRecord {
            id: "d".into(),
            display_name: "d".into(),
            connection: ConnectionState::Disconnected,
            digital_target: None,
            analog_target: None,
            lan_endpoint: None,
            identity: Some(
                json!({"firmware": {"build_id": "b", "build_profile": "release", "features": ["net_http"]}}),
            ),
            usb_pd_cache: None,
            status_cache: None,
            control_cache: None,
            status_meta_cache: None,
            status_cache_updated_at_ms: None,
            usb_status_cache: None,
            usb_status_generation: 0,
            usb_status_sampled_at_ms: None,
            usb_status_source: None,
            selected_artifact_id: None,
            log_decode: LogDecodeState::default(),
            logs: VecDeque::new(),
            trace: VecDeque::new(),
        };
        let artifact = FirmwareArtifact {
            artifact_id: "a".into(),
            name: "artifact".into(),
            target: TargetKind::DigitalEsp32s3,
            package_version: "0.1.0".into(),
            git_sha: "abc".into(),
            build_id: "b".into(),
            build_profile: "release".into(),
            features: vec!["net_http".into()],
            protocol: "p".into(),
            defmt: DefmtMetadata {
                enabled: true,
                encoding: "defmt".into(),
                elf_sha256: None,
                table_sha256: None,
            },
            files: vec![],
        };
        apply_artifact_match(&mut device, Some(&artifact));
        assert_eq!(device.log_decode.status, "verified");
    }

    #[test]
    fn default_digital_usb_port_creates_single_digital_candidate() {
        let candidates = scan_serial_targets(Some("/dev/cu.usbmodem212101"));
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].port_path.as_deref(),
            Some("/dev/cu.usbmodem212101")
        );
        assert_eq!(candidates[0].kind, TargetKind::DigitalEsp32s3);
        assert_eq!(
            candidates[0].selector_source.as_deref(),
            Some(DEFAULT_DIGITAL_USB_PORT_SELECTOR_SOURCE)
        );
    }

    #[test]
    fn default_digital_usb_port_memory_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        write_default_digital_usb_port(dir.path(), " /dev/cu.usbmodem212101 ").unwrap();
        assert_eq!(
            read_default_digital_usb_port(dir.path()).as_deref(),
            Some("/dev/cu.usbmodem212101")
        );
    }

    #[test]
    fn default_digital_usb_port_reads_historical_metadata_record() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            default_digital_usb_port_path(dir.path()),
            "/dev/cu.usbmodem212101\nmac=b8:f8:62:d6:86:38\n",
        )
        .unwrap();
        assert_eq!(
            read_default_digital_usb_port(dir.path()).as_deref(),
            Some("/dev/cu.usbmodem212101")
        );
    }

    #[test]
    fn default_analog_probe_selector_reads_plain_selector_record() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            default_analog_probe_path(dir.path()),
            "0483:3748:ABCDEF\nserial=ignored\n",
        )
        .unwrap();

        assert_eq!(
            read_default_analog_probe_selector(dir.path()).as_deref(),
            Some("0483:3748:ABCDEF")
        );
    }

    #[tokio::test]
    async fn scan_attaches_default_analog_probe_to_default_digital_device() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            default_digital_usb_port_path(dir.path()),
            "/dev/cu.usbmodem212101\n",
        )
        .unwrap();
        fs::write(default_analog_probe_path(dir.path()), "0483:3748:ABCDEF\n").unwrap();
        let state = AppState::new(dir.path().to_path_buf());

        let _ = scan_devices(State(state.clone())).await.unwrap();

        let guard = state.inner.lock().expect("state lock");
        let digital = guard
            .devices
            .values()
            .find(|device| {
                device
                    .digital_target
                    .as_ref()
                    .and_then(|target| target.selector_source.as_deref())
                    == Some(DEFAULT_DIGITAL_USB_PORT_SELECTOR_SOURCE)
            })
            .expect("digital device");
        assert_eq!(
            digital
                .digital_target
                .as_ref()
                .and_then(|target| target.port_path.as_deref()),
            Some("/dev/cu.usbmodem212101")
        );
        assert_eq!(
            digital
                .analog_target
                .as_ref()
                .and_then(|target| target.probe_selector.as_deref()),
            Some("0483:3748:ABCDEF")
        );
        assert_eq!(
            digital
                .analog_target
                .as_ref()
                .and_then(|target| target.selector_source.as_deref()),
            Some(DEFAULT_ANALOG_PROBE_SELECTOR_SOURCE)
        );
    }

    #[test]
    fn serial_json_extractor_recovers_frame_after_log_noise() {
        let frames = extract_serial_json_frames(
            "\u{fffd}\0binary log {\"type\":\"response\",\"request_id\":\"devd-get-identity\",\"ok\":true}\n",
        );

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].frame.get("request_id").and_then(Value::as_str),
            Some("devd-get-identity")
        );
        assert!(frames[0].non_protocol_bytes > 0);
    }

    #[test]
    fn serial_json_extractor_keeps_clean_frame_clean() {
        let frames = extract_serial_json_frames(
            "{\"type\":\"response\",\"request_id\":\"devd-output-disable\",\"ok\":true}",
        );

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].frame.get("request_id").and_then(Value::as_str),
            Some("devd-output-disable")
        );
        assert_eq!(frames[0].non_protocol_bytes, 0);
    }

    #[test]
    fn serial_json_extractor_prefers_outer_response_frame() {
        let frames = extract_serial_json_frames(
            "noise {\"type\":\"response\",\"request_id\":\"devd-get-status\",\"ok\":true,\"data\":{\"status\":{\"enable\":false}}}",
        );

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].frame.get("request_id").and_then(Value::as_str),
            Some("devd-get-status")
        );
        assert!(frames[0].frame.get("data").is_some());
    }

    #[test]
    fn serial_response_inference_recovers_ok_fragment() {
        let probe = SerialProtocolProbe {
            frames: Vec::new(),
            non_protocol_bytes: 80,
            non_protocol_text:
                "noise {\"type\":\"response\",\"request_id\":\"devd-output-disable\",\"ok\":true,"
                    .to_string(),
        };

        let response = infer_serial_response_from_text(&probe, "devd-output-disable").unwrap();
        assert_eq!(response.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            response.get("recovered_from_text").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn serial_response_inference_accepts_generated_output_ids() {
        let request_id = "devd-set-output-enabled-123456";
        let probe = SerialProtocolProbe {
            frames: Vec::new(),
            non_protocol_bytes: 96,
            non_protocol_text: format!(
                "noise {{\"type\":\"response\",\"request_id\":\"{request_id}\",\"ok\":true,"
            ),
        };

        let response = infer_serial_response_from_text(&probe, request_id).unwrap();
        assert_eq!(
            response.get("request_id").and_then(Value::as_str),
            Some(request_id)
        );
        assert_eq!(response.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            response
                .pointer("/data/output_enabled")
                .and_then(Value::as_bool),
            None
        );
    }

    #[test]
    fn serial_response_inference_recovers_fragmented_output_disable_payload() {
        let request_id = "devd-set-output-enabled-123456";
        let probe = SerialProtocolProbe {
            frames: vec![SerialProtocolFrame {
                direction: "tx",
                frame: json!({
                    "type": "request",
                    "request_id": request_id,
                    "op": "set_output_enabled",
                    "enable": false
                }),
            }],
            non_protocol_bytes: 128,
            non_protocol_text: format!(
                "noise {{\"type\":\"response\",\"request_id\":\"{request_id}\",\"ok\":true,\"data\":{{\"output_enabled\":false,\"changed\""
            ),
        };

        let response = infer_serial_response_from_text(&probe, request_id).unwrap();
        assert_eq!(
            response
                .pointer("/data/output_enabled")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn status_response_inference_recovers_fragmented_status_payload() {
        let request_id = "devd-get-status-123456";
        let probe = SerialProtocolProbe {
            frames: vec![
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": "get_status"}),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "active_preset_id": 1,
                        "mode": "cc",
                        "output_enabled": true,
                        "target_i_ma": 2000,
                        "target_v_mv": 0,
                        "target_p_mw": 0,
                        "min_v_mv": 0
                    }),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "state_flags": 6,
                        "fault_flags": 0,
                        "enable": true,
                        "i_local_ma": 998,
                        "i_remote_ma": 986,
                        "v_local_mv": 11845,
                        "v_remote_mv": -858,
                        "calc_p_mw": 23500
                    }),
                },
            ],
            non_protocol_bytes: 128,
            non_protocol_text: String::new(),
        };

        let response = infer_serial_response_from_fragments(&probe, request_id).unwrap();
        assert_eq!(
            response.get("request_id").and_then(Value::as_str),
            Some(request_id)
        );
        let data = status_data_from_serial_response(Some(response)).unwrap();
        assert_eq!(data["status"]["enable"], true);
        assert_eq!(data["control"]["target_i_ma"], 2000);
        assert_eq!(data["recovered_from_fragments"], true);
    }

    #[test]
    fn status_response_inference_recovers_embedded_status_payload() {
        let request_id = "devd-get-status-123456";
        let probe = SerialProtocolProbe {
            frames: vec![
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": "get_status"}),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "analog_state": "ready",
                        "control": {
                            "active_preset_id": 1,
                            "mode": "cc",
                            "output_enabled": false,
                            "target_i_ma": 2000
                        },
                        "hello_seen": true,
                        "link_up": true,
                        "status": {
                            "state_flags": 2,
                            "fault_flags": 0,
                            "enable": false,
                            "i_local_ma": 11,
                            "i_remote_ma": 8,
                            "v_local_mv": 12046,
                            "v_remote_mv": -876,
                            "calc_p_mw": 228
                        }
                    }),
                },
            ],
            non_protocol_bytes: 128,
            non_protocol_text: String::new(),
        };

        let response = infer_serial_response_from_fragments(&probe, request_id).unwrap();
        let data = status_data_from_serial_response(Some(response)).unwrap();
        assert_eq!(data["status"]["enable"], false);
        assert_eq!(data["control"]["output_enabled"], false);
        assert_eq!(data["recovered_from_fragments"], true);
    }

    #[test]
    fn control_response_inference_recovers_standalone_preset_payload() {
        let request_id = "devd-get-control-123456";
        let probe = SerialProtocolProbe {
            frames: vec![
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": "get_control"}),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "preset_id": 1,
                        "mode": "cp",
                        "target_i_ma": 0,
                        "target_v_mv": 0,
                        "target_p_mw": 20000,
                        "min_v_mv": 0,
                        "max_i_ma_total": 5000,
                        "max_p_mw": 200000
                    }),
                },
            ],
            non_protocol_bytes: 128,
            non_protocol_text: String::new(),
        };

        let response = infer_serial_response_from_fragments(&probe, request_id).unwrap();
        let data = serial_response_data(response, "USB control GET").unwrap();
        assert_eq!(data["active_preset_id"], 1);
        assert_eq!(data["preset"]["mode"], "cp");
        assert_eq!(data["preset"]["target_p_mw"], 20000);
        assert_eq!(data["recovered_from_fragments"], true);
    }

    #[test]
    fn presets_response_inference_recovers_standalone_preset_payloads() {
        let request_id = "devd-get-presets-123456";
        let probe = SerialProtocolProbe {
            frames: vec![
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": "get_presets"}),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "preset_id": 2,
                        "mode": "cp",
                        "target_i_ma": 0,
                        "target_v_mv": 0,
                        "target_p_mw": 90000,
                        "min_v_mv": 0,
                        "max_i_ma_total": 5000,
                        "max_p_mw": 200000
                    }),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "preset_id": 1,
                        "mode": "cp",
                        "target_i_ma": 0,
                        "target_v_mv": 0,
                        "target_p_mw": 20000,
                        "min_v_mv": 0,
                        "max_i_ma_total": 5000,
                        "max_p_mw": 200000
                    }),
                },
            ],
            non_protocol_bytes: 128,
            non_protocol_text: String::new(),
        };

        let response = infer_serial_response_from_fragments(&probe, request_id).unwrap();
        let data = serial_response_data(response, "USB presets GET").unwrap();
        assert_eq!(data["presets"][0]["preset_id"], 1);
        assert_eq!(data["presets"][1]["preset_id"], 2);
        assert_eq!(data["recovered_from_fragments"], true);
    }

    #[test]
    fn preset_recovery_merges_partial_retry_results() {
        let mut merged = HashMap::new();
        assert!(merge_presets_from_data(
            &mut merged,
            &json!({
                "presets": [
                    {"preset_id": 2, "mode": "cp"},
                    {"preset_id": 3, "mode": "cc"}
                ],
                "recovered_from_fragments": true
            })
        ));
        assert!(merge_presets_from_data(
            &mut merged,
            &json!({
                "presets": [
                    {"preset_id": 1, "mode": "cp"},
                    {"preset_id": 2, "mode": "cv"}
                ],
                "recovered_from_fragments": true
            })
        ));

        let data = presets_data_from_map(&merged, true, false);
        assert_eq!(data["presets"][0]["preset_id"], 1);
        assert_eq!(data["presets"][1]["preset_id"], 2);
        assert_eq!(data["presets"][1]["mode"], "cv");
        assert_eq!(data["presets"][2]["preset_id"], 3);
        assert_eq!(data["recovered_by_retry"], true);
    }

    #[test]
    fn preset_recovery_merges_control_preset() {
        let mut merged = HashMap::new();
        assert!(merge_presets_from_data(
            &mut merged,
            &json!({
                "presets": [
                    {"preset_id": 2, "mode": "cp", "max_i_ma_total": 5000, "max_p_mw": 200000},
                    {"preset_id": 3, "mode": "cc", "max_i_ma_total": 1500, "max_p_mw": 3000}
                ]
            })
        ));
        assert!(merge_presets_from_data(
            &mut merged,
            &json!({
                "active_preset_id": 1,
                "preset": {
                    "preset_id": 1,
                    "mode": "cp",
                    "target_i_ma": 0,
                    "target_v_mv": 0,
                    "target_p_mw": 20000,
                    "min_v_mv": 0,
                    "max_i_ma_total": 5000,
                    "max_p_mw": 200000
                }
            })
        ));

        let data = presets_data_from_map(&merged, true, true);
        assert_eq!(data["presets"][0]["preset_id"], 1);
        assert_eq!(data["presets"][1]["preset_id"], 2);
        assert_eq!(data["presets"][2]["preset_id"], 3);
        assert_eq!(data["recovered_by_control"], true);
    }

    #[test]
    fn calibration_profile_inference_recovers_compact_payload_from_text() {
        let request_id = "devd-get-calibration-profile-123456";
        let probe = SerialProtocolProbe {
            frames: vec![SerialProtocolFrame {
                direction: "tx",
                frame: json!({"type": "request", "request_id": request_id, "op": "get_calibration_profile"}),
            }],
            non_protocol_bytes: 128,
            non_protocol_text: String::from(
                "noise {\"type\":\"response\",\"request_id\":\"devd-get-calibration-profile-12\"a\":[\"user-calibrated\",3,42],\"c1\":[[2310,299,439]],\"c2\":[[2620,340,493]],\"vl\":[[821,522]],\"vr\":[[828,522]]}}\n",
            ),
        };

        let response = infer_serial_response_from_text(&probe, request_id).unwrap();
        let data = serial_response_data(response, "USB calibration profile").unwrap();
        let expanded = expand_compact_calibration_profile(data).unwrap();
        assert_eq!(expanded["active"]["source"], "user-calibrated");
        assert_eq!(expanded["active"]["fmt_version"], 3);
        assert_eq!(expanded["current_ch1_points"][0]["meas_ma"], 439);
        assert_eq!(expanded["v_remote_points"][0]["meas_mv"], 522);
    }

    #[test]
    fn wifi_status_inference_recovers_state_from_text() {
        let request_id = "devd-get-wifi-status-123456";
        let probe = SerialProtocolProbe {
            frames: vec![SerialProtocolFrame {
                direction: "tx",
                frame: json!({"type": "request", "request_id": request_id, "op": "get_wifi_status"}),
            }],
            non_protocol_bytes: 128,
            non_protocol_text: String::from(
                "noise {\"type\":\"response\",\"request_id\":\"devd-get-wifi-status-12tate\":\"connected\",\"ip\":\"192.168.31.216\",\"last_error\":null}}\n",
            ),
        };

        let response = infer_serial_response_from_text(&probe, request_id).unwrap();
        let data = serial_response_data(response, "USB WiFi status").unwrap();
        assert_eq!(data["state"], "connected");
        assert_eq!(data["ip"], "192.168.31.216");
        assert_eq!(data["last_error"], Value::Null);
        assert_eq!(data["ssid"], Value::Null);
        assert_eq!(data["recovered_from_text"], true);
    }

    #[test]
    fn identity_response_inference_recovers_embedded_firmware_payload() {
        let request_id = "devd-get-identity-123456";
        let probe = SerialProtocolProbe {
            frames: vec![
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": "get_identity"}),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "device_id": "loadlynx-a1b2c3",
                        "hostname": "loadlynx-a1b2c3.local",
                        "short_id": "a1b2c3"
                    }),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "build_id": "digital 0.1.0 (profile release, src 0x1)",
                        "build_profile": "release",
                        "features": ["net_http", "usb_cdc_jsonl"],
                        "protocol": "loadlynx.cdc.v1",
                        "target": "digital_esp32s3"
                    }),
                },
            ],
            non_protocol_bytes: 128,
            non_protocol_text: String::new(),
        };

        let response = infer_serial_response_from_fragments(&probe, request_id).unwrap();
        let identity = identity_data_from_serial_response(Some(response)).unwrap();
        assert_eq!(identity["device_id"], "loadlynx-a1b2c3");
        assert_eq!(
            identity["firmware"]["build_id"],
            "digital 0.1.0 (profile release, src 0x1)"
        );
        assert_eq!(identity["recovered_from_fragments"], true);
        assert_eq!(identity["stable_identity"]["short_id"], "a1b2c3");
    }

    #[test]
    fn identity_response_inference_recovers_stable_identity_without_firmware_payload() {
        let request_id = "devd-get-identity-123456";
        let probe = SerialProtocolProbe {
            frames: vec![
                SerialProtocolFrame {
                    direction: "tx",
                    frame: json!({"type": "request", "request_id": request_id, "op": "get_identity"}),
                },
                SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "device_id": "loadlynx-a1b2c3",
                        "hostname": "loadlynx-a1b2c3.local",
                        "short_id": "a1b2c3"
                    }),
                },
            ],
            non_protocol_bytes: 128,
            non_protocol_text: String::new(),
        };

        let response = infer_serial_response_from_fragments(&probe, request_id).unwrap();
        let identity = identity_data_from_serial_response(Some(response)).unwrap();
        assert_eq!(identity["device_id"], "loadlynx-a1b2c3");
        assert_eq!(identity["firmware_version"], "digital unknown");
        assert_eq!(identity["firmware"]["target"], "digital_esp32s3");
        assert_eq!(identity["recovered_from_fragments"], true);
        assert_eq!(identity["stable_identity"]["short_id"], "a1b2c3");
    }

    #[test]
    fn status_response_extractor_requires_real_status_payload() {
        let data = status_data_from_serial_response(Some(json!({
            "type": "response",
            "request_id": "devd-get-status",
            "ok": true,
            "data": {
                "status": {
                    "enable": true,
                    "v_local_mv": 5010,
                    "i_local_ma": 120,
                    "fault_flags": 0
                }
            }
        })))
        .unwrap();

        assert_eq!(data["status"]["enable"], true);
        assert_eq!(data["status"]["v_local_mv"], 5010);

        let legacy = status_data_from_serial_response(Some(json!({
            "type": "response",
            "request_id": "devd-get-status",
            "ok": true,
            "status": {
                "enable": false,
                "v_local_mv": 9049,
                "i_local_ma": 9,
                "fault_flags": 0
            },
            "control": {
                "mode": "cp",
                "output_enabled": false
            },
            "dat_state": "ready"
        })))
        .unwrap();

        assert_eq!(legacy["status"]["enable"], false);
        assert_eq!(legacy["control"]["mode"], "cp");
        assert_eq!(legacy["dat_state"], "ready");

        let err = status_data_from_serial_response(Some(json!({
            "type": "response",
            "request_id": "devd-get-status",
            "ok": true,
            "data": {}
        })))
        .unwrap_err();
        assert_eq!(err.0.code, "serial_response_invalid");
    }

    #[test]
    fn identity_response_extractor_requires_real_identity_payload() {
        let identity = identity_data_from_serial_response(Some(json!({
            "type": "response",
            "request_id": "devd-get-identity",
            "ok": true,
            "data": {
                "device_id": "digital-esp32s3",
                "target": "digital",
                "mcu": "esp32s3",
                "protocol": "loadlynx.cdc.v1"
            }
        })))
        .unwrap();

        assert_eq!(identity["device_id"], "digital-esp32s3");
        assert_eq!(identity["target"], "digital");

        let err = identity_data_from_serial_response(Some(json!({
            "type": "response",
            "request_id": "devd-get-identity",
            "ok": true,
            "data": {}
        })))
        .unwrap_err();
        assert_eq!(err.0.code, "serial_response_invalid");
    }

    #[test]
    fn serial_response_data_rejects_protocol_errors() {
        let err = serial_response_data(
            json!({
                "type": "error",
                "request_id": "devd-output-enable",
                "ok": false,
                "error": {
                    "code": "LINK_DOWN",
                    "message": "UART link is down"
                }
            }),
            "USB CC control",
        )
        .unwrap_err();

        assert_eq!(err.1, StatusCode::CONFLICT);
        assert_eq!(err.0.code, "LINK_DOWN");
        assert_eq!(err.0.message, "UART link is down");
    }

    #[test]
    fn pd_post_requires_fresh_protocol_response() {
        let err = pd_post_response_data(None).unwrap_err();
        assert_eq!(err.0.code, "serial_response_missing");
    }

    #[tokio::test]
    async fn pd_get_uses_cache_after_response_gap_errors() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard
                .devices
                .get_mut("mock-loadlynx-devd")
                .unwrap()
                .usb_pd_cache = Some(json!({"attached": true, "cached": true}));
        }

        let Json(cached) = pd_cache_or_serial_error(
            &state,
            "mock-loadlynx-devd",
            HttpError::retryable(
                "serial_response_timeout",
                "USB request did not receive a matching response",
            ),
        )
        .unwrap();

        assert_eq!(cached["attached"], true);
        assert_eq!(cached["cached"], true);
    }

    #[tokio::test]
    async fn pd_get_does_not_cache_fallback_for_serial_open_errors() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard
                .devices
                .get_mut("mock-loadlynx-devd")
                .unwrap()
                .usb_pd_cache = Some(json!({"attached": true}));
        }

        let err = pd_cache_or_serial_error(
            &state,
            "mock-loadlynx-devd",
            HttpError::retryable("serial_open_failed", "port unavailable"),
        )
        .unwrap_err();

        assert_eq!(err.0.code, "serial_open_failed");
    }

    #[test]
    fn trace_text_sanitizer_removes_control_characters() {
        let sanitized = sanitize_trace_text("a\0b\u{1b}c\n");
        assert_eq!(sanitized, "a\\x00b\\x1bc\\x0a");
        assert!(!sanitized.chars().any(char::is_control));
    }

    #[tokio::test]
    async fn multiple_active_leases_require_explicit_selection() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            let mut second = guard.devices.get("mock-loadlynx-devd").unwrap().clone();
            second.id = "mock-loadlynx-devd-2".to_string();
            guard.devices.insert(second.id.clone(), second);
            guard.leases.insert(
                "lease-1".to_string(),
                WebLease {
                    lease_id: "lease-1".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some("mock://digital".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
            guard.leases.insert(
                "lease-2".to_string(),
                WebLease {
                    lease_id: "lease-2".to_string(),
                    device_id: "mock-loadlynx-devd-2".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some("mock://digital-2".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );

            let err = select_compat_device(&guard, None, None).unwrap_err();
            assert_eq!(err.0.code, "device_selection_required");
        }
    }

    #[tokio::test]
    async fn same_device_leases_are_unambiguous_for_compat_selection() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            for lease_id in ["lease-1", "lease-2"] {
                guard.leases.insert(
                    lease_id.to_string(),
                    WebLease {
                        lease_id: lease_id.to_string(),
                        device_id: "mock-loadlynx-devd".to_string(),
                        identity_device_id: None,
                        bind_probe: false,
                        legacy_preflash_only: false,
                        port_path: Some("mock://esp32s3".to_string()),
                        expires_at: Instant::now() + Duration::from_secs(30),
                    },
                );
            }

            let device = select_compat_device(&guard, None, None).unwrap();
            assert_eq!(device.id, "mock-loadlynx-devd");
        }
    }

    #[tokio::test]
    async fn compat_status_prefers_fresh_cached_usb_status() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            let device = guard.devices.get_mut("mock-loadlynx-devd").unwrap();
            device.usb_status_cache = Some(json!({
                "device_id": "mock-loadlynx-devd",
                "analog_state": "ready",
                "control": {"output_enabled": true, "target_i_ma": 1500},
                "status": {"enable": true, "v_local_mv": 11888, "i_local_ma": 742, "fault_flags": 0}
            }));
            device.usb_status_generation = 41;
            device.usb_status_sampled_at_ms = Some(now_unix_ms());
            device.usb_status_source = Some("unit_test_cache".to_string());
            guard.leases.insert(
                "lease-1".to_string(),
                WebLease {
                    lease_id: "lease-1".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some("mock://esp32s3".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
        }

        let Json(status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: None,
                lease_id: Some("lease-1".to_string()),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();

        assert_eq!(status["status_sample_generation"], 41);
        assert_eq!(status["status_sample_source"], "unit_test_cache");
        assert_eq!(status["status"]["v_local_mv"], 11888);

        let guard = state.inner.lock().expect("state lock");
        assert!(
            guard.devices["mock-loadlynx-devd"]
                .trace
                .iter()
                .all(|trace| trace.direction != "tx")
        );
    }

    #[tokio::test]
    async fn bind_probe_lease_is_restricted_to_identity_binding() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.leases.insert(
                "bind-probe".to_string(),
                WebLease {
                    lease_id: "bind-probe".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: Some("mock-loadlynx-devd".to_string()),
                    bind_probe: true,
                    legacy_preflash_only: false,
                    port_path: Some("mock://esp32s3".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );

            let err =
                ensure_lease_for_target(&guard, Some("mock-loadlynx-devd"), Some("bind-probe"))
                    .unwrap_err();
            assert_eq!(err.0.code, "bind_probe_lease_restricted");

            let err = select_serial_port_for_compat(
                &guard,
                &CompatQuery {
                    device_id: None,
                    lease_id: Some("bind-probe".to_string()),
                    fresh: false,
                    cache: false,
                },
                "status",
            )
            .unwrap_err();
            assert_eq!(err.0.code, "bind_probe_lease_restricted");

            let (device_id, port_path) = select_serial_port_for_compat(
                &guard,
                &CompatQuery {
                    device_id: None,
                    lease_id: Some("bind-probe".to_string()),
                    fresh: false,
                    cache: false,
                },
                "identity",
            )
            .unwrap();
            assert_eq!(device_id, "mock-loadlynx-devd");
            assert_eq!(port_path, "mock://esp32s3");
        }
    }

    #[tokio::test]
    async fn legacy_preflash_lease_is_restricted_to_digital_flash() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.leases.insert(
                "legacy-preflash".to_string(),
                WebLease {
                    lease_id: "legacy-preflash".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: Some("digital-esp32s3".to_string()),
                    bind_probe: false,
                    legacy_preflash_only: true,
                    port_path: Some("mock://esp32s3".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );

            let err = ensure_lease_for_target(
                &guard,
                Some("mock-loadlynx-devd"),
                Some("legacy-preflash"),
            )
            .unwrap_err();
            assert_eq!(err.0.code, "legacy_preflash_lease_restricted");

            ensure_flash_lease_for_target(
                &guard,
                Some("mock-loadlynx-devd"),
                Some("legacy-preflash"),
                &TargetKind::DigitalEsp32s3,
            )
            .unwrap();

            let err = ensure_flash_lease_for_target(
                &guard,
                Some("mock-loadlynx-devd"),
                Some("legacy-preflash"),
                &TargetKind::AnalogStm32g431,
            )
            .unwrap_err();
            assert_eq!(err.0.code, "legacy_preflash_lease_restricted");
        }
    }

    #[tokio::test]
    async fn failed_lease_validation_does_not_connect_device() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            let mut other = guard.devices.get("mock-loadlynx-devd").unwrap().clone();
            other.id = "other-device".to_string();
            guard.devices.insert(other.id.clone(), other);
            guard.leases.insert(
                "other-lease".to_string(),
                WebLease {
                    lease_id: "other-lease".to_string(),
                    device_id: "other-device".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some("mock://esp32s3".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
        }

        let err = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.0.code, "device_port_in_use");
        let guard = state.inner.lock().expect("state lock");
        assert_eq!(
            guard.devices.get("mock-loadlynx-devd").unwrap().connection,
            ConnectionState::Disconnected
        );
    }

    #[tokio::test]
    async fn create_lease_rejects_exclusive_port_reservation() {
        let state = AppState::new(PathBuf::from("."));
        mark_serial_exclusive(&state, "mock://esp32s3", "digital flash").unwrap();

        let err = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap_err();

        clear_serial_exclusive(&state, "mock://esp32s3");
        assert_eq!(err.0.code, "operation_in_progress");
        let guard = state.inner.lock().expect("state lock");
        assert_eq!(
            guard.devices.get("mock-loadlynx-devd").unwrap().connection,
            ConnectionState::Disconnected
        );
    }

    #[tokio::test]
    async fn create_lease_rejects_real_port_without_approved_default() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().to_path_buf());
        {
            let mut guard = state.inner.lock().expect("state lock");
            let target = guard
                .devices
                .get_mut("mock-loadlynx-devd")
                .unwrap()
                .digital_target
                .as_mut()
                .unwrap();
            target.port_path = Some("/dev/cu.usbmodem-test".to_string());
            target.selector_source = Some("serialport scan".to_string());
        }

        let err = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.0.code, "target_selector_not_cached");
        let guard = state.inner.lock().expect("state lock");
        assert!(guard.leases.is_empty());
        assert_eq!(
            guard.devices.get("mock-loadlynx-devd").unwrap().connection,
            ConnectionState::Disconnected
        );
    }

    #[tokio::test]
    async fn expected_identity_lease_still_requires_approved_real_port() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().to_path_buf());
        {
            let mut guard = state.inner.lock().expect("state lock");
            let target = guard
                .devices
                .get_mut("mock-loadlynx-devd")
                .unwrap()
                .digital_target
                .as_mut()
                .unwrap();
            target.port_path = Some("/dev/cu.usbmodem-test".to_string());
            target.selector_source = Some("serialport scan".to_string());
        }

        let err = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: Some("loadlynx-abc123".to_string()),
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.0.code, "target_selector_not_cached");
        let guard = state.inner.lock().expect("state lock");
        assert!(guard.leases.is_empty());
        assert_eq!(
            guard.devices.get("mock-loadlynx-devd").unwrap().connection,
            ConnectionState::Disconnected
        );
    }

    #[test]
    fn legacy_preflash_identity_fallback_is_narrow() {
        let input = LeaseRequest {
            device_id: "digital-1".to_string(),
            expected_identity_device_id: Some("digital-esp32s3".to_string()),
            bind_probe: None,
            allow_legacy_preflash_identity_fallback: Some(true),
        };
        let timeout =
            HttpError::retryable("serial_response_timeout", "identity response timed out");
        let open_failed = HttpError::retryable("serial_open_failed", "serial port open failed");
        assert!(allows_legacy_preflash_identity_fallback(&input, &timeout));
        assert!(!allows_legacy_preflash_identity_fallback(
            &input,
            &open_failed
        ));

        let implicit_input = LeaseRequest {
            allow_legacy_preflash_identity_fallback: None,
            ..input.clone()
        };
        assert!(!allows_legacy_preflash_identity_fallback(
            &implicit_input,
            &timeout
        ));

        let stable_input = LeaseRequest {
            expected_identity_device_id: Some("loadlynx-a1b2c3".to_string()),
            ..input.clone()
        };
        assert!(allows_legacy_preflash_identity_fallback(
            &stable_input,
            &timeout
        ));

        let mock_input = LeaseRequest {
            expected_identity_device_id: Some("mock-loadlynx-devd".to_string()),
            ..input.clone()
        };
        assert!(allows_legacy_preflash_identity_fallback(
            &mock_input,
            &timeout
        ));

        let uppercase_input = LeaseRequest {
            expected_identity_device_id: Some("loadlynx-A1B2C3".to_string()),
            ..input.clone()
        };
        assert!(!allows_legacy_preflash_identity_fallback(
            &uppercase_input,
            &timeout
        ));

        let unstable_input = LeaseRequest {
            expected_identity_device_id: Some("loadlynx-bench".to_string()),
            ..input.clone()
        };
        assert!(!allows_legacy_preflash_identity_fallback(
            &unstable_input,
            &timeout
        ));

        let unrelated_input = LeaseRequest {
            expected_identity_device_id: Some("not-stable".to_string()),
            ..input
        };
        assert!(!allows_legacy_preflash_identity_fallback(
            &unrelated_input,
            &timeout
        ));
    }

    #[tokio::test]
    async fn create_lease_rejects_unavailable_real_port() {
        let dir = tempfile::tempdir().unwrap();
        let unavailable_port = dir.path().join("missing-serial-port");
        let unavailable_port = unavailable_port.to_string_lossy().to_string();
        write_default_digital_usb_port(dir.path(), &unavailable_port).unwrap();
        let state = AppState::new(dir.path().to_path_buf());
        {
            let mut guard = state.inner.lock().expect("state lock");
            let target = guard
                .devices
                .get_mut("mock-loadlynx-devd")
                .unwrap()
                .digital_target
                .as_mut()
                .unwrap();
            target.port_path = Some(unavailable_port);
            target.selector_source = Some("serialport scan".to_string());
        }

        let err = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.0.code, "serial_open_failed");
        let guard = state.inner.lock().expect("state lock");
        assert!(guard.leases.is_empty());
        assert_eq!(
            guard.devices.get("mock-loadlynx-devd").unwrap().connection,
            ConnectionState::Disconnected
        );
        assert!(
            state
                .serial
                .lock()
                .expect("serial registry lock")
                .owners
                .is_empty()
        );
    }

    #[tokio::test]
    async fn lease_conflict_uses_removed_device_port_snapshot() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.devices.remove("mock-loadlynx-devd");
            guard.leases.insert(
                "removed-device-lease".to_string(),
                WebLease {
                    lease_id: "removed-device-lease".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some("mock://esp32s3".to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
        }

        let err =
            validate_port_not_leased_by_other_device(&state, "other-device", "mock://esp32s3")
                .unwrap_err();
        assert_eq!(err.0.code, "device_port_in_use");
    }

    #[tokio::test]
    async fn prelease_identity_conflict_stops_serial_owner() {
        let state = AppState::new(PathBuf::from("."));
        let port_path = "/dev/cu.usbmodem-prelease-conflict";
        let sender = serial_owner_sender(&state, port_path).expect("serial owner");
        drop(sender);
        {
            let mut guard = state.inner.lock().expect("state lock");
            let mut other = guard.devices.get("mock-loadlynx-devd").unwrap().clone();
            other.id = "other-device".to_string();
            other.identity = Some(json!({"device_id": "firmware-device-1"}));
            guard.devices.insert(other.id.clone(), other);
        }

        let err = update_device_identity_for_lease_probe(
            &state,
            "mock-loadlynx-devd",
            port_path,
            json!({"device_id": "firmware-device-1"}),
        )
        .unwrap_err();

        assert_eq!(err.0.code, "device_identity_conflict");
        let registry = state.serial.lock().expect("serial registry lock");
        assert!(!registry.owners.contains_key(&canonical_port_key(port_path)));
    }

    #[tokio::test]
    async fn same_device_allows_multiple_leases_and_shared_cached_status() {
        let state = AppState::new(PathBuf::from("."));
        let Json(first) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let Json(second) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();

        let first_lease = first["lease_id"].as_str().unwrap().to_string();
        let second_lease = second["lease_id"].as_str().unwrap().to_string();
        assert_ne!(first_lease, second_lease);
        let trace_len_before = {
            let guard = state.inner.lock().expect("state lock");
            guard.devices.get("mock-loadlynx-devd").unwrap().trace.len()
        };

        let Json(first_status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(first_lease),
                fresh: false,
                cache: false,
            }),
        )
        .await
        .unwrap();
        let Json(second_status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(second_lease),
                fresh: false,
                cache: false,
            }),
        )
        .await
        .unwrap();

        let guard = state.inner.lock().expect("state lock");
        let request_ids = guard
            .devices
            .get("mock-loadlynx-devd")
            .unwrap()
            .trace
            .iter()
            .skip(trace_len_before)
            .filter(|trace| trace.direction == "tx")
            .filter_map(|trace| trace.payload.get("request_id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(request_ids.len(), 2);
        assert_eq!(first_status["device_id"], "mock-loadlynx-devd");
        assert_eq!(second_status["device_id"], "mock-loadlynx-devd");
        assert_eq!(second_status["from_monitor_cache"], serde_json::Value::Null);
        let device = guard.devices.get("mock-loadlynx-devd").unwrap();
        assert!(device.usb_status_generation >= 1);
        assert!(device.usb_status_cache.is_some());
    }

    #[tokio::test]
    async fn compat_status_cache_opt_in_reuses_recent_status_sample() {
        let state = AppState::new(PathBuf::from("."));
        let Json(first) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let Json(second) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();

        let first_lease = first["lease_id"].as_str().unwrap().to_string();
        let second_lease = second["lease_id"].as_str().unwrap().to_string();
        let trace_len_before = {
            let guard = state.inner.lock().expect("state lock");
            guard.devices.get("mock-loadlynx-devd").unwrap().trace.len()
        };

        let Json(first_status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(first_lease),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();
        let Json(second_status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(second_lease),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();

        let guard = state.inner.lock().expect("state lock");
        let request_ids = guard
            .devices
            .get("mock-loadlynx-devd")
            .unwrap()
            .trace
            .iter()
            .skip(trace_len_before)
            .filter(|trace| trace.direction == "tx")
            .filter_map(|trace| trace.payload.get("request_id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(request_ids.len(), 1);
        assert_eq!(first_status["device_id"], "mock-loadlynx-devd");
        assert_eq!(second_status["device_id"], "mock-loadlynx-devd");
        assert_eq!(second_status["from_monitor_cache"], true);
    }

    #[tokio::test]
    async fn compat_status_cache_preserves_offline_link_state() {
        let state = AppState::new(PathBuf::from("."));
        let Json(lease) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap().to_string();
        {
            let mut queued = state
                .mock_serial_responses
                .lock()
                .expect("mock serial responses lock");
            queued.clear();
            queued.push_back(SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "analog_state": "offline",
                        "hello_seen": false,
                        "link_up": false,
                        "status": {
                            "state_flags": 0,
                            "fault_flags": 0,
                            "enable": false,
                            "i_local_ma": 0,
                            "v_local_mv": 0
                        }
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            });
        }

        let Json(first_status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(lease_id.clone()),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();
        let Json(second_status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(lease_id),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();

        assert_eq!(first_status["link_up"], false);
        assert_eq!(second_status["from_monitor_cache"], true);
        assert_eq!(second_status["link_up"], false);
        assert_eq!(second_status["hello_seen"], false);
        assert_eq!(second_status["analog_state"], "offline");
        let guard = state.inner.lock().expect("state lock");
        let device = guard.devices.get("mock-loadlynx-devd").unwrap();
        assert!(device.usb_status_generation >= 1);
        assert!(device.usb_status_cache.is_some());
    }

    #[tokio::test]
    async fn compat_status_retries_after_response_gap_error() {
        let state = AppState::new(PathBuf::from("."));
        let Json(lease) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap().to_string();
        let trace_len_before = {
            let guard = state.inner.lock().expect("state lock");
            guard.devices.get("mock-loadlynx-devd").unwrap().trace.len()
        };
        {
            let mut queued = state
                .mock_serial_responses
                .lock()
                .expect("mock serial responses lock");
            queued.clear();
            queued.push_back(SerialProtocolProbe {
                frames: vec![],
                non_protocol_bytes: 64,
                non_protocol_text: "binary noise only\n".to_string(),
            });
            queued.push_back(SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "analog_state": "ready",
                        "control": {
                            "active_preset_id": 1,
                            "mode": "cc",
                            "output_enabled": false,
                            "target_i_ma": 2000
                        },
                        "hello_seen": true,
                        "link_up": true,
                        "status": {
                            "state_flags": 2,
                            "fault_flags": 0,
                            "enable": false,
                            "i_local_ma": 11,
                            "i_remote_ma": 8,
                            "v_local_mv": 12046,
                            "v_remote_mv": -876,
                            "calc_p_mw": 228
                        }
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            });
        }

        let Json(status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(lease_id),
                fresh: false,
                cache: false,
            }),
        )
        .await
        .unwrap();

        assert_eq!(status["status"]["enable"], false);
        assert_eq!(status["control"]["target_i_ma"], 2000);

        let guard = state.inner.lock().expect("state lock");
        let tx_request_ids = guard
            .devices
            .get("mock-loadlynx-devd")
            .unwrap()
            .trace
            .iter()
            .skip(trace_len_before)
            .filter(|trace| trace.direction == "tx")
            .filter_map(|trace| trace.payload.get("request_id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(tx_request_ids.len(), 2);
        assert_ne!(tx_request_ids[0], tx_request_ids[1]);
    }

    #[tokio::test]
    async fn compat_status_refreshes_stale_cache_after_half_second() {
        let state = AppState::new(PathBuf::from("."));
        let Json(lease) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap().to_string();
        {
            let mut guard = state.inner.lock().expect("state lock");
            let device = guard.devices.get_mut("mock-loadlynx-devd").unwrap();
            device.status_cache = Some(json!({
                "enable": true,
                "fault_flags": 99,
                "i_local_ma": 1,
                "state_flags": 7,
                "v_local_mv": 42
            }));
            device.control_cache = Some(json!({
                "active_preset_id": 9,
                "mode": "cv",
                "output_enabled": true
            }));
            device.status_cache_updated_at_ms =
                Some(Utc::now().timestamp_millis() - STATUS_CACHE_MAX_AGE_MS - 1);
        }
        {
            let mut queued = state
                .mock_serial_responses
                .lock()
                .expect("mock serial responses lock");
            queued.clear();
            queued.push_back(SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "analog_state": "ready",
                        "control": {
                            "active_preset_id": 1,
                            "mode": "cc",
                            "output_enabled": false,
                            "target_i_ma": 2000
                        },
                        "hello_seen": true,
                        "link_up": true,
                        "status": {
                            "state_flags": 2,
                            "fault_flags": 0,
                            "enable": false,
                            "i_local_ma": 11,
                            "i_remote_ma": 8,
                            "v_local_mv": 12046,
                            "v_remote_mv": -876,
                            "calc_p_mw": 228
                        }
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            });
        }

        let trace_len_before = {
            let guard = state.inner.lock().expect("state lock");
            guard.devices.get("mock-loadlynx-devd").unwrap().trace.len()
        };
        let Json(status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(lease_id),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();

        assert_eq!(status["status"]["enable"], false);
        assert_eq!(status["control"]["target_i_ma"], 2000);
        assert_eq!(status["cache_age_ms"], serde_json::Value::Null);

        let guard = state.inner.lock().expect("state lock");
        let tx_request_ids = guard
            .devices
            .get("mock-loadlynx-devd")
            .unwrap()
            .trace
            .iter()
            .skip(trace_len_before)
            .filter(|trace| trace.direction == "tx")
            .filter_map(|trace| trace.payload.get("request_id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(tx_request_ids.len(), 1);
    }

    #[tokio::test]
    async fn compat_status_refresh_without_control_preserves_control_cache() {
        let state = AppState::new(PathBuf::from("."));
        let Json(lease) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap().to_string();
        {
            let mut guard = state.inner.lock().expect("state lock");
            let device = guard.devices.get_mut("mock-loadlynx-devd").unwrap();
            device.status_cache = Some(json!({
                "enable": true,
                "fault_flags": 0,
                "i_local_ma": 1,
                "state_flags": 1,
                "v_local_mv": 12000
            }));
            device.control_cache = Some(json!({
                "active_preset_id": 9,
                "mode": "cc",
                "output_enabled": true,
                "target_i_ma": 1000
            }));
            device.status_cache_updated_at_ms =
                Some(Utc::now().timestamp_millis() - STATUS_CACHE_MAX_AGE_MS - 1);
        }
        {
            let mut queued = state
                .mock_serial_responses
                .lock()
                .expect("mock serial responses lock");
            queued.clear();
            queued.push_back(SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "hello_seen": true,
                        "link_up": true,
                        "status": {
                            "state_flags": 2,
                            "fault_flags": 0,
                            "enable": false,
                            "i_local_ma": 11,
                            "i_remote_ma": 8,
                            "v_local_mv": 12046,
                            "v_remote_mv": -876,
                            "calc_p_mw": 228
                        }
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            });
        }

        let Json(status) = compat_status(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(lease_id),
                fresh: false,
                cache: true,
            }),
        )
        .await
        .unwrap();

        assert_eq!(status["status"]["enable"], false);
        assert_eq!(status["control"]["target_i_ma"], 1000);
        let guard = state.inner.lock().expect("state lock");
        assert_eq!(
            guard.devices["mock-loadlynx-devd"].control_cache,
            Some(json!({
                "active_preset_id": 9,
                "mode": "cc",
                "output_enabled": true,
                "target_i_ma": 1000
            }))
        );
    }

    #[tokio::test]
    async fn compat_control_get_retries_after_response_gap_error() {
        let state = AppState::new(PathBuf::from("."));
        let Json(lease) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap().to_string();
        let trace_len_before = {
            let guard = state.inner.lock().expect("state lock");
            guard.devices.get("mock-loadlynx-devd").unwrap().trace.len()
        };
        {
            let mut queued = state
                .mock_serial_responses
                .lock()
                .expect("mock serial responses lock");
            queued.clear();
            queued.push_back(SerialProtocolProbe {
                frames: vec![],
                non_protocol_bytes: 32,
                non_protocol_text: "dropped control response\n".to_string(),
            });
            queued.push_back(SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "active_preset_id": 1,
                        "output_enabled": false,
                        "uv_latched": false,
                        "preset": {
                            "preset_id": 1,
                            "mode": "cc",
                            "target_i_ma": 2000,
                            "target_v_mv": 0,
                            "target_p_mw": 0,
                            "min_v_mv": 0,
                            "max_i_ma_total": 10000,
                            "max_p_mw": 120000
                        }
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            });
        }

        let Json(control) = compat_control_get(
            State(state.clone()),
            Query(CompatQuery {
                device_id: Some("mock-loadlynx-devd".to_string()),
                lease_id: Some(lease_id),
                fresh: false,
                cache: false,
            }),
        )
        .await
        .unwrap();

        assert_eq!(control["active_preset_id"], 1);
        assert_eq!(control["preset"]["target_i_ma"], 2000);

        let guard = state.inner.lock().expect("state lock");
        let tx_request_ids = guard
            .devices
            .get("mock-loadlynx-devd")
            .unwrap()
            .trace
            .iter()
            .skip(trace_len_before)
            .filter(|trace| trace.direction == "tx")
            .filter_map(|trace| trace.payload.get("request_id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(tx_request_ids.len(), 2);
        assert_ne!(tx_request_ids[0], tx_request_ids[1]);
    }

    #[tokio::test]
    async fn serial_owner_rejects_commands_during_exclusive_operation() {
        let state = AppState::new(PathBuf::from("."));
        mark_serial_exclusive(&state, "/dev/cu.usbmodem-test", "digital flash").unwrap();
        let err = serial_owner_jsonl_request(
            &state,
            "mock-loadlynx-devd",
            "/dev/cu.usbmodem-test",
            "get_status",
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "operation_in_progress");
        clear_serial_exclusive(&state, "/dev/cu.usbmodem-test");
    }

    #[tokio::test]
    async fn session_reads_do_not_take_serial_operation_lock() {
        let state = AppState::new(PathBuf::from("."));
        let Json(lease) = create_lease(
            State(state.clone()),
            Json(LeaseRequest {
                device_id: "mock-loadlynx-devd".to_string(),
                expected_identity_device_id: None,
                bind_probe: None,
                allow_legacy_preflash_identity_fallback: None,
            }),
        )
        .await
        .unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap().to_string();

        mark_serial_exclusive(&state, "mock://esp32s3", "digital flash").unwrap();
        let Json(session) = compat_session(
            State(state.clone()),
            Query(SessionQuery {
                device_id: None,
                lease_id: Some(lease_id),
                logs_limit: None,
                trace_limit: None,
            }),
        )
        .await
        .unwrap();
        clear_serial_exclusive(&state, "mock://esp32s3");

        assert_eq!(session["connected"], true);
        assert!(session["logs"].is_array());
        assert!(session["trace"].is_array());
    }

    #[test]
    fn response_matching_rejects_mismatched_request_id() {
        let probe = SerialProtocolProbe {
            frames: vec![SerialProtocolFrame {
                direction: "rx",
                frame: json!({
                    "type": "response",
                    "request_id": "other-request",
                    "ok": true,
                    "data": {}
                }),
            }],
            non_protocol_bytes: 0,
            non_protocol_text: String::new(),
        };

        assert!(serial_response_for_request(&probe, "wanted-request").is_none());
        assert!(serial_probe_has_mismatched_response(
            &probe,
            "wanted-request"
        ));
    }

    #[tokio::test]
    async fn generated_pd_request_ids_refresh_pd_cache() {
        let state = AppState::new(PathBuf::from("."));
        record_serial_protocol_probe(
            &state,
            "mock-loadlynx-devd",
            "mock://esp32s3",
            "USB PD GET completed",
            SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "type": "response",
                        "request_id": "devd-get-pd-123456",
                        "ok": true,
                        "data": {"attached": true}
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            },
        );

        let guard = state.inner.lock().expect("state lock");
        let cached = guard
            .devices
            .get("mock-loadlynx-devd")
            .and_then(|device| device.usb_pd_cache.as_ref())
            .unwrap();
        assert_eq!(cached["attached"], true);
    }

    #[tokio::test]
    async fn expired_lease_is_released_and_disconnects_device() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            let device = guard.devices.get_mut("mock-loadlynx-devd").unwrap();
            device.connection = ConnectionState::Connected;
            guard.leases.insert(
                "expired".to_string(),
                WebLease {
                    lease_id: "expired".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some("mock://digital".to_string()),
                    expires_at: Instant::now() - Duration::from_secs(1),
                },
            );
        }

        cleanup_expired_leases(&state);
        let guard = state.inner.lock().expect("state lock");
        assert!(!guard.leases.contains_key("expired"));
        assert_eq!(
            guard.devices.get("mock-loadlynx-devd").unwrap().connection,
            ConnectionState::Disconnected
        );
    }

    #[tokio::test]
    async fn expired_lease_stops_serial_owner_after_device_removed() {
        let state = AppState::new(PathBuf::from("."));
        let port_path = "mock://removed-device";
        let sender = serial_owner_sender(&state, port_path).expect("serial owner");
        drop(sender);
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.devices.remove("mock-loadlynx-devd");
            guard.leases.insert(
                "expired".to_string(),
                WebLease {
                    lease_id: "expired".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some(port_path.to_string()),
                    expires_at: Instant::now() - Duration::from_secs(1),
                },
            );
        }

        cleanup_expired_leases(&state);

        let registry = state.serial.lock().expect("serial registry lock");
        assert!(!registry.owners.contains_key(&canonical_port_key(port_path)));
    }

    #[tokio::test]
    async fn serial_owner_sender_rejects_exclusive_reservation() {
        let state = AppState::new(PathBuf::from("."));
        let port_path = "mock://exclusive-race";
        let _exclusive =
            reserve_serial_exclusive(&state, port_path, "digital firmware flash").unwrap();

        match serial_owner_sender(&state, port_path) {
            Ok(_) => panic!("exclusive reservation should reject serial owner creation"),
            Err(error) => {
                assert_eq!(error.0.code, "operation_in_progress");
                assert_eq!(error.1, StatusCode::CONFLICT);
            }
        }
    }

    #[tokio::test]
    async fn exclusive_release_restarts_owner_for_active_port_lease() {
        let state = AppState::new(PathBuf::from("."));
        let port_path = "mock://exclusive-resume";
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.leases.insert(
                "lease-1".to_string(),
                WebLease {
                    lease_id: "lease-1".to_string(),
                    device_id: "mock-loadlynx-devd".to_string(),
                    identity_device_id: None,
                    bind_probe: false,
                    legacy_preflash_only: false,
                    port_path: Some(port_path.to_string()),
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
        }
        let sender = serial_owner_sender(&state, port_path).expect("serial owner");
        drop(sender);

        {
            let _exclusive =
                reserve_serial_exclusive(&state, port_path, "digital firmware flash").unwrap();
            let registry = state.serial.lock().expect("serial registry lock");
            assert!(!registry.owners.contains_key(&canonical_port_key(port_path)));
        }

        let registry = state.serial.lock().expect("serial registry lock");
        assert!(registry.owners.contains_key(&canonical_port_key(port_path)));
    }

    #[tokio::test]
    async fn releasing_one_removed_device_lease_keeps_owner_for_other_port_lease() {
        let state = AppState::new(PathBuf::from("."));
        let port_path = "mock://shared-removed-device";
        let sender = serial_owner_sender(&state, port_path).expect("serial owner");
        drop(sender);
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.devices.remove("mock-loadlynx-devd");
            for lease_id in ["lease-1", "lease-2"] {
                guard.leases.insert(
                    lease_id.to_string(),
                    WebLease {
                        lease_id: lease_id.to_string(),
                        device_id: "mock-loadlynx-devd".to_string(),
                        identity_device_id: None,
                        bind_probe: false,
                        legacy_preflash_only: false,
                        port_path: Some(port_path.to_string()),
                        expires_at: Instant::now() + Duration::from_secs(30),
                    },
                );
            }
        }

        assert!(release_lease_inner(&state, "lease-1", "released"));
        {
            let registry = state.serial.lock().expect("serial registry lock");
            assert!(registry.owners.contains_key(&canonical_port_key(port_path)));
        }

        assert!(release_lease_inner(&state, "lease-2", "released"));
        let registry = state.serial.lock().expect("serial registry lock");
        assert!(!registry.owners.contains_key(&canonical_port_key(port_path)));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn unix_ipc_startup_refuses_live_socket() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loadlynx-devd.sock");
        let _listener = tokio::net::UnixListener::bind(&path).unwrap();

        let err = remove_stale_unix_socket(&path).await.unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
        assert!(path.exists());
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn unix_ipc_startup_removes_stale_socket() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loadlynx-devd.sock");
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        drop(listener);

        remove_stale_unix_socket(&path).await.unwrap();

        assert!(!path.exists());
    }

    #[tokio::test]
    async fn resolve_operation_rejects_artifact_target_mismatch() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert(
                "digital".to_string(),
                test_artifact("digital", TargetKind::DigitalEsp32s3),
            );
        }

        let err = resolve_operation(
            &state,
            "mock-loadlynx-devd",
            Some(TargetKind::AnalogStm32g431),
            Some("digital".to_string()),
            true,
        )
        .unwrap_err();
        assert_eq!(err.0.code, "artifact_target_mismatch");
    }

    #[tokio::test]
    async fn real_flash_requires_matching_lease() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert(
                "digital".to_string(),
                test_artifact("digital", TargetKind::DigitalEsp32s3),
            );
        }

        let err = flash_device(
            State(state),
            Path("mock-loadlynx-devd".to_string()),
            Json(FlashRequest {
                target: Some(TargetKind::DigitalEsp32s3),
                artifact_id: Some("digital".to_string()),
                dry_run: Some(false),
                lease_id: None,
                confirmation_phrase: Some(FLASH_CONFIRMATION_TEXT.to_string()),
                expected_identity_device_id: None,
                acknowledge_non_project_firmware: Some(true),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "web_session_required");
    }

    #[tokio::test]
    async fn real_digital_flash_requires_confirmation_text() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert(
                "digital".to_string(),
                test_artifact("digital", TargetKind::DigitalEsp32s3),
            );
        }

        let err = flash_device(
            State(state),
            Path("mock-loadlynx-devd".to_string()),
            Json(FlashRequest {
                target: Some(TargetKind::DigitalEsp32s3),
                artifact_id: Some("digital".to_string()),
                dry_run: Some(false),
                lease_id: None,
                confirmation_phrase: None,
                expected_identity_device_id: None,
                acknowledge_non_project_firmware: Some(true),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "flash_confirmation_required");
    }

    #[tokio::test]
    async fn real_analog_flash_requires_confirmation_text() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert(
                "analog".to_string(),
                test_artifact("analog", TargetKind::AnalogStm32g431),
            );
        }

        let err = flash_device(
            State(state),
            Path("mock-loadlynx-devd".to_string()),
            Json(FlashRequest {
                target: Some(TargetKind::AnalogStm32g431),
                artifact_id: Some("analog".to_string()),
                dry_run: Some(false),
                lease_id: None,
                confirmation_phrase: None,
                expected_identity_device_id: None,
                acknowledge_non_project_firmware: Some(true),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "operation_confirmation_required");
    }

    #[tokio::test]
    async fn real_analog_flash_requires_non_project_acknowledgement() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut artifact = test_artifact("foreign", TargetKind::AnalogStm32g431);
            artifact.name = "Foreign firmware".to_string();
            artifact.protocol = "unknown".to_string();
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert("foreign".to_string(), artifact);
        }

        let err = flash_device(
            State(state),
            Path("mock-loadlynx-devd".to_string()),
            Json(FlashRequest {
                target: Some(TargetKind::AnalogStm32g431),
                artifact_id: Some("foreign".to_string()),
                dry_run: Some(false),
                lease_id: None,
                confirmation_phrase: Some(FLASH_CONFIRMATION_TEXT.to_string()),
                expected_identity_device_id: None,
                acknowledge_non_project_firmware: Some(false),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "non_project_firmware_ack_required");
    }

    #[tokio::test]
    async fn real_analog_reset_requires_confirmation_text() {
        let state = AppState::new(PathBuf::from("."));

        let err = reset_device(
            State(state),
            Path("mock-loadlynx-devd".to_string()),
            Some(Json(ResetRequest {
                target: Some(TargetKind::AnalogStm32g431),
                dry_run: Some(false),
                lease_id: None,
                confirmation_phrase: None,
            })),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "operation_confirmation_required");
    }

    #[tokio::test]
    async fn real_digital_flash_requires_non_project_acknowledgement() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut artifact = test_artifact("foreign", TargetKind::DigitalEsp32s3);
            artifact.name = "Foreign firmware".to_string();
            artifact.protocol = "unknown".to_string();
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert("foreign".to_string(), artifact);
        }

        let err = flash_device(
            State(state),
            Path("mock-loadlynx-devd".to_string()),
            Json(FlashRequest {
                target: Some(TargetKind::DigitalEsp32s3),
                artifact_id: Some("foreign".to_string()),
                dry_run: Some(false),
                lease_id: None,
                confirmation_phrase: Some(FLASH_CONFIRMATION_TEXT.to_string()),
                expected_identity_device_id: None,
                acknowledge_non_project_firmware: Some(false),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "non_project_firmware_ack_required");
    }

    #[tokio::test]
    async fn real_operation_requires_cached_selector_source() {
        let state = AppState::new(PathBuf::from("."));
        let guard = state.inner.lock().expect("state lock");
        let device = guard.devices.get("mock-loadlynx-devd").unwrap();
        let err = ensure_real_operation_uses_cached_target(device, &TargetKind::DigitalEsp32s3)
            .unwrap_err();
        assert_eq!(err.0.code, "target_selector_not_cached");
    }

    #[test]
    fn analog_probe_operations_do_not_require_usb_lease() {
        assert!(target_requires_usb_lease(&TargetKind::DigitalEsp32s3));
        assert!(!target_requires_usb_lease(&TargetKind::AnalogStm32g431));
    }

    #[tokio::test]
    async fn serial_probe_persists_identity_and_refreshes_artifact_match() {
        let state = AppState::new(PathBuf::from("."));
        {
            let mut guard = state.inner.lock().expect("state lock");
            guard.artifacts.insert(
                "digital".to_string(),
                FirmwareArtifact {
                    artifact_id: "digital".into(),
                    name: "digital".into(),
                    target: TargetKind::DigitalEsp32s3,
                    package_version: "0.1.0".into(),
                    git_sha: "abc".into(),
                    build_id: "digital-build".into(),
                    build_profile: "release".into(),
                    features: vec!["net_http".into(), "usb_cdc_jsonl".into()],
                    protocol: "loadlynx.cdc.v1".into(),
                    defmt: DefmtMetadata {
                        enabled: true,
                        encoding: "defmt-espflash".into(),
                        elf_sha256: None,
                        table_sha256: None,
                    },
                    files: vec![],
                },
            );
            let device = guard.devices.get_mut("mock-loadlynx-devd").unwrap();
            device.selected_artifact_id = Some("digital".to_string());
        }

        record_serial_protocol_probe(
            &state,
            "mock-loadlynx-devd",
            "/dev/cu.usbmodem212101",
            "serial protocol probe completed",
            SerialProtocolProbe {
                frames: vec![SerialProtocolFrame {
                    direction: "rx",
                    frame: json!({
                        "type": "response",
                        "request_id": "devd-get-identity",
                        "ok": true,
                        "data": {
                            "device_id": "digital-esp32s3",
                            "firmware": {
                                "build_id": "digital-build",
                                "build_profile": "release",
                                "features": ["net_http", "usb_cdc_jsonl"]
                            }
                        }
                    }),
                }],
                non_protocol_bytes: 0,
                non_protocol_text: String::new(),
            },
        );

        let guard = state.inner.lock().expect("state lock");
        let device = guard.devices.get("mock-loadlynx-devd").unwrap();
        assert_eq!(
            device
                .identity
                .as_ref()
                .and_then(|identity| identity.get("device_id"))
                .and_then(Value::as_str),
            Some("digital-esp32s3")
        );
        assert_eq!(device.log_decode.status, "verified");
    }
}
