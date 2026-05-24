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
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::broadcast};
use tower_http::{cors::CorsLayer, services::ServeDir};

pub const DEFAULT_BIND: &str = "127.0.0.1:30180";
pub const DEFAULT_DEVD_URL: &str = "http://127.0.0.1:30180";
pub const WEB_LEASE_HEARTBEAT_INTERVAL_MS: u64 = 2_000;
pub const WEB_LEASE_TTL_MS: u64 = 8_000;
const EVENT_LIMIT: usize = 1_000;
const LOG_LIMIT: usize = 500;
const TRACE_LIMIT: usize = 2_000;

#[derive(Debug, Clone)]
pub struct DevdConfig {
    pub bind: SocketAddr,
    pub web_root: Option<PathBuf>,
    pub allow_dev_cors: bool,
    pub repo_root: PathBuf,
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
    events: broadcast::Sender<DevdEvent>,
    repo_root: PathBuf,
}

#[derive(Debug, Default)]
struct DevdState {
    devices: HashMap<String, DeviceRecord>,
    artifacts: HashMap<String, FirmwareArtifact>,
    leases: HashMap<String, WebLease>,
    events: VecDeque<DevdEvent>,
}

#[derive(Debug, Clone)]
struct WebLease {
    lease_id: String,
    device_id: String,
    identity_device_id: Option<String>,
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

impl TargetKind {
    fn board_name(&self) -> Option<&'static str> {
        match self {
            Self::DigitalEsp32s3 => Some("digital"),
            Self::AnalogStm32g431 => Some("analog"),
            Self::LanHttp | Self::Mock => None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub id: String,
    pub display_name: String,
    pub connection: ConnectionState,
    pub digital_target: Option<TargetCandidate>,
    pub analog_target: Option<TargetCandidate>,
    pub lan_endpoint: Option<String>,
    pub identity: Option<Value>,
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
    Artifact(FirmwareArtifact),
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
}

#[derive(Debug, Deserialize)]
struct ResetRequest {
    target: Option<TargetKind>,
    dry_run: Option<bool>,
    lease_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LeaseRequest {
    device_id: String,
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    lease_id: Option<String>,
    logs_limit: Option<usize>,
    trace_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CompatQuery {
    device_id: Option<String>,
    lease_id: Option<String>,
}

pub async fn serve(config: DevdConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let state = AppState::new(config.repo_root.clone());
    let router = router(state, config.web_root, config.allow_dev_cors);
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!("loadlynx-devd listening on http://{}", config.bind);
    axum::serve(listener, router).await?;
    Ok(())
}

impl AppState {
    pub fn new(repo_root: PathBuf) -> Self {
        let (events, _) = broadcast::channel(EVENT_LIMIT);
        let state = Self {
            inner: Arc::new(Mutex::new(DevdState::default())),
            events,
            repo_root,
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
        .with_state(state);

    if allow_dev_cors {
        router = router.layer(
            CorsLayer::new()
                .allow_origin([
                    HeaderValue::from_static("http://localhost:5173"),
                    HeaderValue::from_static("http://127.0.0.1:5173"),
                ])
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(tower_http::cors::Any),
        );
    }

    if let Some(web_root) = web_root {
        router = router.fallback_service(ServeDir::new(web_root));
    }
    router
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
    discovered.extend(scan_serial_targets());
    discovered.extend(scan_cached_selector_targets(&state.repo_root));

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
                selected_artifact_id: None,
                log_decode: LogDecodeState::default(),
                logs: VecDeque::new(),
                trace: VecDeque::new(),
            });
        match candidate.kind {
            TargetKind::DigitalEsp32s3 => entry.digital_target = Some(candidate),
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
    if let Some(Json(input)) = body {
        if input.identity.is_some() {
            device.identity = input.identity;
        }
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
    Ok(Json(json!({
        "device_id": id,
        "connection": device.connection,
        "targets": {
            "digital": device.digital_target,
            "analog": device.analog_target,
            "lan": device.lan_endpoint
        },
        "log_decode": device.log_decode
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
        input.target,
        input.artifact_id,
        input.dry_run.unwrap_or(true),
    )?;
    verify_artifact_files(&artifact)?;
    if dry_run {
        return Ok(Json(
            json!({"ok": true, "dry_run": true, "action": "flash", "target_evidence": evidence}),
        ));
    }
    {
        let guard = state.inner.lock().expect("state lock");
        ensure_lease_for_target(&guard, Some(&id), input.lease_id.as_deref())?;
        let device = guard
            .devices
            .get(&id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        ensure_agentd_uses_selected_target(device, &target)?;
    }
    run_agentd(target.clone(), "flash").await?;
    emit(
        &state,
        Some(id),
        "flash",
        "firmware flash completed",
        json!({"artifact_id": artifact.artifact_id, "target": target}),
    );
    Ok(Json(
        json!({"ok": true, "dry_run": false, "action": "flash", "target_evidence": evidence}),
    ))
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
    });
    let target = input.target.unwrap_or(TargetKind::DigitalEsp32s3);
    let dry_run = input.dry_run.unwrap_or(true);
    let evidence = target_evidence(&state, &id, target.clone(), None)?;
    if dry_run {
        return Ok(Json(
            json!({"ok": true, "dry_run": true, "action": "reset", "target_evidence": evidence}),
        ));
    }
    {
        let guard = state.inner.lock().expect("state lock");
        ensure_lease_for_target(&guard, Some(&id), input.lease_id.as_deref())?;
        let device = guard
            .devices
            .get(&id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
        ensure_agentd_uses_selected_target(device, &target)?;
    }
    run_agentd(target.clone(), "reset").await?;
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
        if let Some(existing) = guard
            .leases
            .values()
            .find(|lease| lease.device_id == input.device_id && lease.expires_at > Instant::now())
        {
            return Err(HttpError::conflict(
                "device_lease_conflict",
                format!(
                    "device already has an active Web lease: {}",
                    existing.lease_id
                ),
            ));
        }
        guard
            .devices
            .get(&input.device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "device is not known"))?;
    }
    let _ = connect_device(State(state.clone()), Path(input.device_id.clone()), None).await?;

    let lease_id = next_id();
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

async fn compat_identity(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = select_compat_device(
        &guard,
        query.lease_id.as_deref(),
        query.device_id.as_deref(),
    )?;
    Ok(Json(device.identity.clone().unwrap_or_else(|| {
        mock_identity(&device.id, &device.display_name)
    })))
}

async fn compat_status(
    State(state): State<AppState>,
    Query(query): Query<CompatQuery>,
) -> Result<Json<Value>, HttpError> {
    let guard = state.inner.lock().expect("state lock");
    let device = select_compat_device(
        &guard,
        query.lease_id.as_deref(),
        query.device_id.as_deref(),
    )?;
    Ok(Json(
        json!({"device_id": device.id, "connection": device.connection, "log_decode": device.log_decode}),
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

async fn compat_events(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    events_stream(state, None)
}

async fn device_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    events_stream(state, Some(id))
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
    let active = state
        .leases
        .values()
        .filter(|lease| lease.expires_at > Instant::now())
        .collect::<Vec<_>>();
    match active.as_slice() {
        [lease] => state
            .devices
            .get(&lease.device_id)
            .ok_or_else(|| HttpError::not_found("device_not_found", "leased device is not known")),
        [] => Err(HttpError::bad_request(
            "web_session_required",
            "Web USB lease or explicit device_id is required",
        )),
        _ => Err(HttpError::bad_request(
            "device_selection_required",
            "multiple Web USB leases are active; specify lease_id or device_id",
        )),
    }
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
    if let Some(target) = target_device_id {
        if lease.device_id != target {
            return Err(HttpError::conflict(
                "device_lease_mismatch",
                "Web USB lease does not match requested device",
            ));
        }
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
    emit(
        state,
        Some(lease.device_id),
        "web_lease",
        &format!("Web USB lease {reason}"),
        json!({"lease_id": lease_id}),
    );
    true
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
            .filter_map(|(id, lease)| (lease.expires_at <= Instant::now()).then(|| id.clone()))
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
        "logs": tail(&device.logs, logs_limit.unwrap_or(200).min(LOG_LIMIT)),
        "trace": tail(&device.trace, trace_limit.unwrap_or(600).min(TRACE_LIMIT)),
    })
}

fn lease_json(lease: &WebLease) -> Value {
    json!({
        "lease_id": lease.lease_id,
        "device_id": lease.device_id,
        "identity_device_id": lease.identity_device_id,
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

fn ensure_agentd_uses_selected_target(
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
        TargetKind::DigitalEsp32s3 => ".esp32-port",
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

async fn run_agentd(target: TargetKind, action: &str) -> Result<(), HttpError> {
    let Some(board) = target.board_name() else {
        return Err(HttpError::bad_request(
            "target_unsupported",
            "target cannot be handled by mcu-agentd",
        ));
    };
    let status = Command::new("just")
        .arg("agentd")
        .arg(action)
        .arg(board)
        .stdin(Stdio::null())
        .status()
        .await
        .map_err(|error| HttpError::retryable("agentd_launch_failed", error.to_string()))?;
    if !status.success() {
        return Err(HttpError::retryable(
            "agentd_failed",
            format!("mcu-agentd {action} {board} exited with {status}"),
        ));
    }
    Ok(())
}

fn read_manifest(path: &str) -> Result<Vec<FirmwareArtifact>, HttpError> {
    let text = fs::read_to_string(path).map_err(|error| {
        HttpError::retryable("artifact_read_failed", format!("{path}: {error}"))
    })?;
    let manifest: FirmwareManifest = serde_json::from_str(&text)
        .map_err(|error| HttpError::bad_request("artifact_parse_failed", error.to_string()))?;
    match manifest {
        FirmwareManifest::Artifact(artifact) => Ok(vec![artifact]),
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

fn scan_serial_targets() -> Vec<TargetCandidate> {
    let mut out = Vec::new();
    let Ok(ports) = serialport::available_ports() else {
        return out;
    };
    for port in ports {
        if is_native_usb_serial_candidate(&port) {
            out.push(TargetCandidate {
                kind: TargetKind::DigitalEsp32s3,
                display_name: format!("ESP32-S3 USB CDC ({})", port.port_name),
                port_path: Some(port.port_name),
                probe_selector: None,
                lan_base_url: None,
                selector_source: Some("serialport scan".to_string()),
            });
        }
    }
    out
}

fn scan_cached_selector_targets(repo_root: &FsPath) -> Vec<TargetCandidate> {
    let mut out = Vec::new();
    let esp32 = repo_root.join(".esp32-port");
    if let Some(port) = read_selector_cache(&esp32) {
        out.push(TargetCandidate {
            kind: TargetKind::DigitalEsp32s3,
            display_name: format!("ESP32-S3 cached selector ({port})"),
            port_path: Some(port),
            probe_selector: None,
            lan_base_url: None,
            selector_source: Some(".esp32-port".to_string()),
        });
    }
    let stm32 = repo_root.join(".stm32-port");
    if let Some(selector) = read_selector_cache(&stm32) {
        out.push(TargetCandidate {
            kind: TargetKind::AnalogStm32g431,
            display_name: format!("STM32G431 cached probe ({selector})"),
            port_path: None,
            probe_selector: Some(selector),
            lan_base_url: None,
            selector_source: Some(".stm32-port".to_string()),
        });
    }
    out
}

fn read_selector_cache(path: &FsPath) -> Option<String> {
    let value = fs::read_to_string(path).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn is_native_usb_serial_candidate(port: &serialport::SerialPortInfo) -> bool {
    let name = port.port_name.to_lowercase();
    matches!(port.port_type, serialport::SerialPortType::UsbPort(_))
        || name.contains("usbmodem")
        || name.contains("usbserial")
        || name.contains("wchusbserial")
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
    let mut redacted = frame.clone();
    if redacted.get("type").and_then(Value::as_str) == Some("wifi_config") {
        if let Some(obj) = redacted.as_object_mut() {
            if obj.contains_key("psk") {
                obj.insert("psk".to_string(), Value::String("<redacted>".to_string()));
            }
        }
    }
    redacted
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
    use std::io::Write;

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
    fn native_usb_serial_candidate_filters_expected_names() {
        assert!(is_native_usb_serial_candidate(&SerialPortInfo {
            port_name: "/dev/cu.usbmodem101".into(),
            port_type: SerialPortType::Unknown,
        }));
        assert!(is_native_usb_serial_candidate(&SerialPortInfo {
            port_name: "COM7".into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x303a,
                pid: 0x1001,
                serial_number: Some("abc".into()),
                manufacturer: None,
                product: None,
            }),
        }));
        assert!(!is_native_usb_serial_candidate(&SerialPortInfo {
            port_name: "/dev/ttys001".into(),
            port_type: SerialPortType::Unknown,
        }));
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
    fn reads_selector_cache_without_writing_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".stm32-port");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "0483:3748:ABC").unwrap();
        assert_eq!(read_selector_cache(&path).unwrap(), "0483:3748:ABC");
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
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
            guard.leases.insert(
                "lease-2".to_string(),
                WebLease {
                    lease_id: "lease-2".to_string(),
                    device_id: "mock-loadlynx-devd-2".to_string(),
                    identity_device_id: None,
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );

            let err = select_compat_device(&guard, None, None).unwrap_err();
            assert_eq!(err.0.code, "device_selection_required");
        }
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
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0.code, "web_session_required");
    }

    #[tokio::test]
    async fn real_agentd_requires_cached_selector_source() {
        let state = AppState::new(PathBuf::from("."));
        let guard = state.inner.lock().expect("state lock");
        let device = guard.devices.get("mock-loadlynx-devd").unwrap();
        let err =
            ensure_agentd_uses_selected_target(device, &TargetKind::DigitalEsp32s3).unwrap_err();
        assert_eq!(err.0.code, "target_selector_not_cached");
    }
}
