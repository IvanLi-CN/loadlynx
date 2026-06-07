use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SavedTransport {
    Usb,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LegacySavedHardware {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    pub(crate) transport: SavedTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) devd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_seen_unix_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SavedHardware {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) identity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_transport: Option<SavedTransport>,
    #[serde(default, skip_serializing_if = "SavedTransports::is_empty")]
    pub(crate) transports: SavedTransports,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_seen_unix_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SavedTransports {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) usb: Option<SavedUsbTransport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) http: Option<SavedHttpTransport>,
}

impl SavedTransports {
    pub(crate) fn is_empty(&self) -> bool {
        self.usb.is_none() && self.http.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SavedUsbTransport {
    pub(crate) device: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) port_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) devd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SavedHttpTransport {
    pub(crate) url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HardwareRegistry {
    #[serde(default = "hardware_registry_schema_version")]
    pub(crate) schema_version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default_hardware_id: Option<String>,
    #[serde(default)]
    pub(crate) hardware: Vec<SavedHardware>,
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
pub(crate) struct ResolvedUsbHardware {
    pub(crate) hardware_id: String,
    pub(crate) device: String,
    pub(crate) devd: String,
    pub(crate) port_path: Option<String>,
    pub(crate) expected_identity_device_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum ResolvedHardware {
    Usb(ResolvedUsbHardware),
    Http { hardware_id: String, url: String },
}

pub(crate) async fn handle_hardware_command(
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

pub(crate) fn resolve_saved_hardware(
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

pub(crate) fn resolve_hardware_transport(
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
                expected_identity_device_id: is_stable_hardware_id(&hardware.id)
                    .then(|| hardware.id.clone()),
            }))
        }
        SavedTransport::Http => {
            let http =
                hardware.transports.http.as_ref().ok_or_else(|| {
                    format!("saved hardware {} has no HTTP transport", hardware.id)
                })?;
            Ok(ResolvedHardware::Http {
                hardware_id: hardware.id.clone(),
                url: http.url.clone(),
            })
        }
    }
}

pub(crate) fn resolve_usb_target(
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

pub(crate) fn mark_default_transport_used(
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

pub(crate) fn mark_hardware_transport_used(
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn upsert_hardware_transport(
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

pub(crate) fn sort_hardware(hardware: &mut [SavedHardware]) {
    hardware.sort_by(|left, right| left.id.cmp(&right.id));
}

pub(crate) fn ensure_hardware_has_transport(
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

pub(crate) fn available_hardware_payload(
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

pub(crate) fn devd_error_payload(error: impl std::fmt::Display) -> Value {
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

pub(crate) fn stable_hardware_id_from_identity(
    identity: &Value,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let id = identity
        .get("device_id")
        .and_then(Value::as_str)
        .ok_or("identity did not include stable device_id")?;
    if is_stable_hardware_id(id) {
        Ok(id.to_string())
    } else {
        Err(format!(
            "identity device_id `{id}` is not a stable LoadLynx hardware id; update firmware before binding or controlling this device"
        )
        .into())
    }
}

pub(crate) fn is_stable_hardware_id(id: &str) -> bool {
    id.strip_prefix("loadlynx-").is_some_and(is_hex_short_id) || id.starts_with("mock-")
}

fn is_hex_short_id(value: &str) -> bool {
    value.len() == 6
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub(crate) fn read_hardware_registry(
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

pub(crate) fn migrate_legacy_hardware_registry(
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

pub(crate) fn write_hardware_registry(
    path: &Path,
    registry: &HardwareRegistry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(registry)?)?;
    Ok(())
}

pub(crate) fn hardware_registry_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>>
{
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

pub(crate) fn hardware_registry_path_from_values(
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
