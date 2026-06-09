use super::*;

const LOCAL_DEVICE_FILE_NAME: &str = ".loadlynx";

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

#[derive(Debug, Clone)]
pub(crate) struct LocalDeviceSelection {
    pub(crate) path: PathBuf,
    pub(crate) id: String,
}

#[derive(Debug, Clone)]
struct InteractiveChoice {
    id: String,
    label: String,
}

#[derive(Debug, Clone)]
struct UsbAddCandidate {
    candidate_id: String,
    display_name: String,
    port_path: Option<String>,
}

pub(crate) async fn handle_device_command(
    command: DeviceCommand,
    client: &Client,
    devd: &str,
    allow_interactive: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let registry_path = hardware_registry_path()?;
    let cwd = env::current_dir()?;

    match command {
        DeviceCommand::List => device_list_payload(&registry_path, &cwd),
        DeviceCommand::Add { url, name } => {
            let mut registry = read_hardware_registry(&registry_path)?;
            let now = current_unix_seconds();
            let saved = if let Some(url) = url {
                bind_http_device(&mut registry, client, &url, name, now).await?
            } else {
                if !allow_interactive {
                    return Err(
                        "`loadlynx device add` without --url requires an interactive terminal"
                            .into(),
                    );
                }
                bind_usb_device_interactive(&mut registry, client, devd, name, now).await?
            };
            if registry.default_hardware_id.is_none() {
                registry.default_hardware_id = Some(saved.id.clone());
            }
            write_hardware_registry(&registry_path, &registry)?;
            Ok(json!({
                "path": registry_path,
                "device": saved,
                "default_device_id": registry.default_hardware_id,
            }))
        }
        DeviceCommand::Use { id, global, clear } => {
            let mut registry = read_hardware_registry(&registry_path)?;
            if clear {
                if id.is_some() {
                    return Err("`loadlynx device use --clear` does not accept a device id".into());
                }
                if global {
                    registry.default_hardware_id = None;
                    write_hardware_registry(&registry_path, &registry)?;
                    return Ok(json!({
                        "scope": "global",
                        "path": registry_path,
                        "cleared": true,
                    }));
                }
                let local_path = local_device_path_for_dir(&cwd);
                let removed = if local_path.exists() {
                    fs::remove_file(&local_path)?;
                    true
                } else {
                    false
                };
                return Ok(json!({
                    "scope": "local",
                    "path": local_path,
                    "cleared": removed,
                }));
            }

            let selected_id = match id {
                Some(id) => id,
                None if allow_interactive => {
                    choose_saved_device_id(&registry, read_local_device_selection(&cwd)?, None)?
                }
                None => return Err(
                    "device use requires <device-id> unless an interactive terminal is available"
                        .into(),
                ),
            };
            let saved = registry
                .hardware
                .iter()
                .find(|hardware| hardware.id == selected_id)
                .cloned()
                .ok_or_else(|| format!("saved device not found: {selected_id}"))?;

            if global {
                registry.default_hardware_id = Some(selected_id.clone());
                write_hardware_registry(&registry_path, &registry)?;
                Ok(json!({
                    "scope": "global",
                    "path": registry_path,
                    "device_id": selected_id,
                    "device": saved,
                }))
            } else {
                let local_path = write_local_device_selection(&cwd, &selected_id)?;
                Ok(json!({
                    "scope": "local",
                    "path": local_path,
                    "device_id": selected_id,
                    "device": saved,
                }))
            }
        }
        DeviceCommand::Remove { id } => {
            let mut registry = read_hardware_registry(&registry_path)?;
            let before = registry.hardware.len();
            registry.hardware.retain(|hardware| hardware.id != id);
            let removed = registry.hardware.len() != before;
            if registry.default_hardware_id.as_deref() == Some(id.as_str()) {
                registry.default_hardware_id = None;
            }
            write_hardware_registry(&registry_path, &registry)?;
            Ok(json!({
                "path": registry_path,
                "device_id": id,
                "removed": removed,
                "default_device_id": registry.default_hardware_id,
            }))
        }
    }
}

fn device_list_payload(
    registry_path: &Path,
    cwd: &Path,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut registry = read_hardware_registry(registry_path)?;
    sort_hardware(&mut registry.hardware);
    let local = read_local_device_selection(cwd)?;
    Ok(json!({
        "path": registry_path,
        "local_device": local.as_ref().map(|selection| json!({
            "path": selection.path,
            "device_id": selection.id,
        })),
        "global_default_device_id": registry.default_hardware_id,
        "devices": registry.hardware,
    }))
}

async fn bind_usb_device_interactive(
    registry: &mut HardwareRegistry,
    client: &Client,
    devd: &str,
    name: Option<String>,
    now: u64,
) -> Result<SavedHardware, Box<dyn std::error::Error + Send + Sync>> {
    let scan =
        request_devd_value(devd, reqwest::Method::POST, "/api/v1/devices/scan", None).await?;
    let candidates = usb_add_candidates_from_scan(&scan)?;
    let selected = choose_usb_add_candidate(&candidates)?;
    let identity = read_usb_identity_for_bind(client, devd, &selected.candidate_id).await?;
    let hardware_id = stable_hardware_id_from_identity(&identity)?;
    let preferred_name = name.or_else(|| Some(selected.display_name.clone()));
    Ok(upsert_hardware_transport(
        registry,
        hardware_id,
        preferred_name,
        Some(identity),
        SavedTransport::Usb,
        Some(SavedUsbTransport {
            device: selected.candidate_id,
            port_path: selected.port_path,
            devd: None,
        }),
        None,
        now,
    ))
}

async fn bind_http_device(
    registry: &mut HardwareRegistry,
    client: &Client,
    url: &str,
    name: Option<String>,
    now: u64,
) -> Result<SavedHardware, Box<dyn std::error::Error + Send + Sync>> {
    let identity =
        request_http_value(client, url, reqwest::Method::GET, "/api/v1/identity", None).await?;
    let hardware_id = stable_hardware_id_from_identity(&identity)?;
    let preferred_name = name.or_else(|| {
        Url::parse(url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_string))
    });
    Ok(upsert_hardware_transport(
        registry,
        hardware_id,
        preferred_name,
        Some(identity),
        SavedTransport::Http,
        None,
        Some(SavedHttpTransport {
            url: url.to_string(),
        }),
        now,
    ))
}

fn usb_add_candidates_from_scan(
    scan: &Value,
) -> Result<Vec<UsbAddCandidate>, Box<dyn std::error::Error + Send + Sync>> {
    let devices = scan
        .get("devices")
        .and_then(Value::as_array)
        .ok_or("device scan response did not include devices")?;
    let mut candidates = Vec::new();
    for device in devices {
        let Some(candidate_id) = device.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(digital_target) = device.get("digital_target") else {
            continue;
        };
        candidates.push(UsbAddCandidate {
            candidate_id: candidate_id.to_string(),
            display_name: device
                .get("display_name")
                .and_then(Value::as_str)
                .or_else(|| digital_target.get("display_name").and_then(Value::as_str))
                .unwrap_or(candidate_id)
                .to_string(),
            port_path: digital_target
                .get("port_path")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    if candidates.is_empty() {
        return Err("no USB candidates found; connect the device and retry".into());
    }
    Ok(candidates)
}

fn choose_usb_add_candidate(
    candidates: &[UsbAddCandidate],
) -> Result<UsbAddCandidate, Box<dyn std::error::Error + Send + Sync>> {
    if candidates.len() == 1 {
        return Ok(candidates[0].clone());
    }

    let items = candidates
        .iter()
        .map(|candidate| match candidate.port_path.as_deref() {
            Some(port_path) => format!("{} ({port_path})", candidate.display_name),
            None => candidate.display_name.clone(),
        })
        .collect::<Vec<_>>();
    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a device to add")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Interrupted, "device add cancelled"))?;
    Ok(candidates[selected].clone())
}

pub(crate) fn resolve_saved_hardware_selection(
    device: Option<String>,
    default_devd: &str,
    allow_interactive: bool,
) -> Result<ResolvedHardware, Box<dyn std::error::Error + Send + Sync>> {
    resolve_saved_hardware_selection_with_transport(device, default_devd, allow_interactive, None)
}

pub(crate) fn resolve_saved_hardware_selection_with_transport(
    device: Option<String>,
    default_devd: &str,
    allow_interactive: bool,
    required_transport: Option<SavedTransport>,
) -> Result<ResolvedHardware, Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let registry = read_hardware_registry(&path)?;
    if let Some(resolved) = resolve_saved_hardware_noninteractive_from_registry(
        &registry,
        device.as_deref(),
        default_devd,
        required_transport,
    )? {
        return Ok(resolved);
    }

    if allow_interactive {
        let cwd = env::current_dir()?;
        let local = read_local_device_selection(&cwd)?;
        let selected_id = choose_saved_device_id(&registry, local, required_transport)?;
        return resolve_saved_hardware_from_registry(
            &selected_id,
            &registry,
            default_devd,
            required_transport,
        );
    }

    match required_transport {
        Some(SavedTransport::Usb) => Err("no device selected; pass --device <saved-id>, run `loadlynx device use <saved-id>`, or set a global default with `loadlynx device use --global <saved-id>`".into()),
        _ => Err("no device selected; pass --device <saved-id>, run `loadlynx device use <saved-id>`, or set a global default with `loadlynx device use --global <saved-id>`".into()),
    }
}

pub(crate) fn has_saved_device_for_transport(
    required_transport: Option<SavedTransport>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let path = hardware_registry_path()?;
    let registry = read_hardware_registry(&path)?;
    Ok(registry.hardware.iter().any(|hardware| {
        required_transport.is_none_or(|transport| match transport {
            SavedTransport::Usb => hardware.transports.usb.is_some(),
            SavedTransport::Http => hardware.transports.http.is_some(),
        })
    }))
}

fn resolve_saved_hardware_noninteractive_from_registry(
    registry: &HardwareRegistry,
    explicit_id: Option<&str>,
    default_devd: &str,
    required_transport: Option<SavedTransport>,
) -> Result<Option<ResolvedHardware>, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(id) = explicit_id {
        return resolve_saved_hardware_from_registry(
            id,
            registry,
            default_devd,
            required_transport,
        )
        .map(Some);
    }

    let cwd = env::current_dir()?;
    if let Some(local) = read_local_device_selection(&cwd)? {
        return resolve_saved_hardware_from_registry(
            &local.id,
            registry,
            default_devd,
            required_transport,
        )
        .map_err(|error| {
            format!(
                "local device file {} points to an unusable saved device: {error}",
                local.path.display()
            )
            .into()
        })
        .map(Some);
    }

    if registry.default_hardware_id.is_some() {
        return resolve_saved_hardware_from_registry(
            "default",
            registry,
            default_devd,
            required_transport,
        )
        .map(Some);
    }

    Ok(None)
}

fn resolve_saved_hardware_from_registry(
    id: &str,
    registry: &HardwareRegistry,
    default_devd: &str,
    required_transport: Option<SavedTransport>,
) -> Result<ResolvedHardware, Box<dyn std::error::Error + Send + Sync>> {
    let resolved_id = if id == "default" {
        registry
            .default_hardware_id
            .as_deref()
            .ok_or("default device is not set; run `loadlynx device use --global <saved-id>`")?
    } else {
        id
    };
    let hardware = registry
        .hardware
        .iter()
        .find(|hardware| hardware.id == resolved_id)
        .ok_or_else(|| format!("saved device not found: {resolved_id}"))?;

    let transport = match required_transport {
        Some(transport) => transport,
        None => hardware.last_transport.ok_or_else(|| {
            format!(
                "saved device {resolved_id} has no selected transport; re-add it or update the registry"
            )
        })?,
    };
    resolve_hardware_transport(hardware, transport, default_devd)
}

fn choose_saved_device_id(
    registry: &HardwareRegistry,
    local: Option<LocalDeviceSelection>,
    required_transport: Option<SavedTransport>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let local_id = local.as_ref().map(|selection| selection.id.as_str());
    let choices = registry
        .hardware
        .iter()
        .filter(|hardware| {
            required_transport.is_none_or(|transport| match transport {
                SavedTransport::Usb => hardware.transports.usb.is_some(),
                SavedTransport::Http => hardware.transports.http.is_some(),
            })
        })
        .map(|hardware| InteractiveChoice {
            id: hardware.id.clone(),
            label: interactive_choice_label(
                hardware,
                registry.default_hardware_id.as_deref(),
                local_id,
            ),
        })
        .collect::<Vec<_>>();

    if choices.is_empty() {
        return Err(match required_transport {
            Some(SavedTransport::Usb) => {
                "no saved USB devices found; run `loadlynx device add`".into()
            }
            Some(SavedTransport::Http) => {
                "no saved HTTP devices found; run `loadlynx device add --url <base-url>`".into()
            }
            None => "no saved devices found; run `loadlynx device add`".into(),
        });
    }

    if choices.len() == 1 {
        return Ok(choices[0].id.clone());
    }

    let items = choices
        .iter()
        .map(|choice| choice.label.as_str())
        .collect::<Vec<_>>();
    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a saved device")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Interrupted, "device selection cancelled"))?;
    Ok(choices[selected].id.clone())
}

fn interactive_choice_label(
    hardware: &SavedHardware,
    global_default: Option<&str>,
    local_id: Option<&str>,
) -> String {
    let mut markers = Vec::new();
    if local_id == Some(hardware.id.as_str()) {
        markers.push("local");
    }
    if global_default == Some(hardware.id.as_str()) {
        markers.push("global");
    }
    if hardware.transports.usb.is_some() {
        markers.push("usb");
    }
    if hardware.transports.http.is_some() {
        markers.push("http");
    }
    if let Some(last) = hardware.last_transport {
        markers.push(match last {
            SavedTransport::Usb => "pref=usb",
            SavedTransport::Http => "pref=http",
        });
    }

    match hardware.name.as_deref() {
        Some(name) => format!("{} ({name}) [{}]", hardware.id, markers.join(", ")),
        None => format!("{} [{}]", hardware.id, markers.join(", ")),
    }
}

pub(crate) fn resolve_hardware_transport(
    hardware: &SavedHardware,
    transport: SavedTransport,
    default_devd: &str,
) -> Result<ResolvedHardware, Box<dyn std::error::Error + Send + Sync>> {
    match transport {
        SavedTransport::Usb => {
            let usb = hardware
                .transports
                .usb
                .as_ref()
                .ok_or_else(|| format!("saved device {} has no USB transport", hardware.id))?;
            Ok(ResolvedHardware::Usb(ResolvedUsbHardware {
                hardware_id: hardware.id.clone(),
                device: usb.device.clone(),
                devd: sanitize_usb_devd_endpoint(usb.devd.as_deref(), default_devd),
                port_path: usb.port_path.clone(),
                expected_identity_device_id: is_stable_hardware_id(&hardware.id)
                    .then(|| hardware.id.clone()),
            }))
        }
        SavedTransport::Http => {
            let http = hardware
                .transports
                .http
                .as_ref()
                .ok_or_else(|| format!("saved device {} has no HTTP transport", hardware.id))?;
            Ok(ResolvedHardware::Http {
                hardware_id: hardware.id.clone(),
                url: http.url.clone(),
            })
        }
    }
}

fn sanitize_usb_devd_endpoint(stored: Option<&str>, default_devd: &str) -> String {
    match stored {
        Some(endpoint) if endpoint.starts_with("http://") || endpoint.starts_with("https://") => {
            default_devd.to_string()
        }
        Some(endpoint) => endpoint.to_string(),
        None => default_devd.to_string(),
    }
}

pub(crate) fn resolve_usb_target(
    device: Option<String>,
    default_devd: &str,
    allow_interactive: bool,
) -> Result<ResolvedUsbHardware, Box<dyn std::error::Error + Send + Sync>> {
    match resolve_saved_hardware_selection_with_transport(
        device,
        default_devd,
        allow_interactive,
        Some(SavedTransport::Usb),
    )? {
        ResolvedHardware::Usb(resolved) => Ok(resolved),
        ResolvedHardware::Http { .. } => {
            Err("selected device resolved to HTTP unexpectedly".into())
        }
    }
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
            Err(format!("saved device {} has no USB transport", hardware.id).into())
        }
        SavedTransport::Http if hardware.transports.http.is_none() => {
            Err(format!("saved device {} has no HTTP transport", hardware.id).into())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
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
        "global_default_device_id": registry.default_hardware_id,
        "scan_requested": scan,
        "scan": scan_result,
        "usb": {
            "devices": devd_devices,
            "remembered": registry.hardware.iter().filter(|hardware| hardware.transports.usb.is_some()).cloned().collect::<Vec<_>>(),
        },
        "http_fallback": registry.hardware.iter().filter(|hardware| hardware.transports.http.is_some()).cloned().collect::<Vec<_>>(),
        "devices": registry.hardware,
    })
}

pub(crate) fn local_device_path_for_dir(dir: &Path) -> PathBuf {
    dir.join(LOCAL_DEVICE_FILE_NAME)
}

pub(crate) fn read_local_device_selection(
    start: &Path,
) -> Result<Option<LocalDeviceSelection>, Box<dyn std::error::Error + Send + Sync>> {
    let Some(path) = find_local_device_path(start) else {
        return Ok(None);
    };
    let id = fs::read_to_string(&path)?;
    let id = id.trim();
    if id.is_empty() {
        return Err(format!("local device file {} is empty", path.display()).into());
    }
    Ok(Some(LocalDeviceSelection {
        path,
        id: id.to_string(),
    }))
}

fn find_local_device_path(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(local_device_path_for_dir)
        .find(|path| path.is_file())
}

fn write_local_device_selection(
    dir: &Path,
    id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let path = local_device_path_for_dir(dir);
    fs::write(&path, format!("{id}\n"))?;
    Ok(path)
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
            "identity device_id `{id}` is not a stable LoadLynx device id; update firmware before binding or controlling this device"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static CWD_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn sample_registry() -> HardwareRegistry {
        let mut registry = HardwareRegistry {
            default_hardware_id: Some("loadlynx-global".to_string()),
            ..HardwareRegistry::default()
        };
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-global".to_string(),
            Some("Global".to_string()),
            None,
            SavedTransport::Usb,
            Some(SavedUsbTransport {
                device: "digital-global".to_string(),
                port_path: Some("mock://global".to_string()),
                devd: None,
            }),
            None,
            1,
        );
        upsert_hardware_transport(
            &mut registry,
            "loadlynx-local".to_string(),
            Some("Local".to_string()),
            None,
            SavedTransport::Usb,
            Some(SavedUsbTransport {
                device: "digital-local".to_string(),
                port_path: Some("mock://local".to_string()),
                devd: None,
            }),
            None,
            2,
        );
        registry
    }

    #[test]
    fn local_device_file_uses_nearest_ancestor() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let parent = root.join("workspace");
        let child = parent.join("nested").join("deeper");
        fs::create_dir_all(&child).unwrap();

        fs::write(local_device_path_for_dir(root), "loadlynx-root\n").unwrap();
        fs::write(local_device_path_for_dir(&parent), "loadlynx-parent\n").unwrap();

        let selection = read_local_device_selection(&child).unwrap().unwrap();
        assert_eq!(selection.id, "loadlynx-parent");
        assert_eq!(selection.path, local_device_path_for_dir(&parent));
    }

    #[test]
    fn explicit_device_overrides_local_and_global_selection() {
        let _guard = CWD_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let child = root.join("project");
        fs::create_dir_all(&child).unwrap();
        fs::write(local_device_path_for_dir(&child), "loadlynx-local\n").unwrap();

        let previous_cwd = env::current_dir().unwrap();
        env::set_current_dir(&child).unwrap();
        let resolved = resolve_saved_hardware_noninteractive_from_registry(
            &sample_registry(),
            Some("loadlynx-global"),
            "http://127.0.0.1:30180",
            Some(SavedTransport::Usb),
        )
        .unwrap()
        .unwrap();
        env::set_current_dir(previous_cwd).unwrap();

        match resolved {
            ResolvedHardware::Usb(resolved) => {
                assert_eq!(resolved.hardware_id, "loadlynx-global");
                assert_eq!(resolved.device, "digital-global");
            }
            ResolvedHardware::Http { .. } => panic!("expected usb device"),
        }
    }

    #[test]
    fn local_device_overrides_global_default_selection() {
        let _guard = CWD_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let child = root.join("project");
        fs::create_dir_all(&child).unwrap();
        fs::write(local_device_path_for_dir(&child), "loadlynx-local\n").unwrap();

        let previous_cwd = env::current_dir().unwrap();
        env::set_current_dir(&child).unwrap();
        let resolved = resolve_saved_hardware_noninteractive_from_registry(
            &sample_registry(),
            None,
            "http://127.0.0.1:30180",
            Some(SavedTransport::Usb),
        )
        .unwrap()
        .unwrap();
        env::set_current_dir(previous_cwd).unwrap();

        match resolved {
            ResolvedHardware::Usb(resolved) => {
                assert_eq!(resolved.hardware_id, "loadlynx-local");
                assert_eq!(resolved.device, "digital-local");
            }
            ResolvedHardware::Http { .. } => panic!("expected usb device"),
        }
    }

    #[test]
    fn saved_usb_legacy_http_devd_endpoint_falls_back_to_default_ipc() {
        let hardware = SavedHardware {
            id: "loadlynx-legacy".to_string(),
            name: None,
            identity: None,
            last_transport: Some(SavedTransport::Usb),
            transports: SavedTransports {
                usb: Some(SavedUsbTransport {
                    device: "digital-legacy".to_string(),
                    port_path: Some("/dev/cu.usbmodem212101".to_string()),
                    devd: Some("http://127.0.0.1:30180".to_string()),
                }),
                http: None,
            },
            last_seen_unix_seconds: None,
        };

        match resolve_hardware_transport(&hardware, SavedTransport::Usb, "/tmp/loadlynx.sock")
            .unwrap()
        {
            ResolvedHardware::Usb(resolved) => {
                assert_eq!(resolved.devd, "/tmp/loadlynx.sock");
                assert_eq!(resolved.device, "digital-legacy");
            }
            ResolvedHardware::Http { .. } => panic!("expected usb device"),
        }
    }
}
