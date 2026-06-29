use super::*;

#[derive(Debug, Clone)]
pub(crate) struct ApiSelector {
    pub(crate) url: Option<String>,
    pub(crate) device: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn request_api_value(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    allow_interactive: bool,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    allow_insecure_lan_wifi: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    ensure_one_api_selector(selector.url.as_ref(), selector.device.as_ref())?;
    let is_wifi_write = path == "/api/v1/wifi"
        && (method == reqwest::Method::POST || method == reqwest::Method::DELETE);
    if let Some(url) = selector.url {
        if is_wifi_write && !allow_insecure_lan_wifi {
            return Err("LAN WiFi writes require --allow-insecure-lan-wifi".into());
        }
        request_http_value(client, &url, method, path, body).await
    } else {
        match resolve_saved_hardware_selection(selector.device, default_devd, allow_interactive)? {
            ResolvedHardware::Usb(resolved) => {
                let value = request_devd_usb_value(client, &resolved, method, path, body).await?;
                let _ = mark_hardware_transport_used(&resolved.hardware_id, SavedTransport::Usb);
                Ok(value)
            }
            ResolvedHardware::Http { hardware_id, url } => {
                if is_wifi_write && !allow_insecure_lan_wifi {
                    return Err("LAN WiFi writes require --allow-insecure-lan-wifi".into());
                }
                let value = request_http_value(client, &url, method, path, body).await?;
                let _ = mark_hardware_transport_used(&hardware_id, SavedTransport::Http);
                Ok(value)
            }
        }
    }
}

pub(crate) fn freeze_api_selector(
    selector: ApiSelector,
    default_devd: &str,
    allow_interactive: bool,
) -> Result<ApiSelector, Box<dyn std::error::Error + Send + Sync>> {
    ensure_one_api_selector(selector.url.as_ref(), selector.device.as_ref())?;
    if selector.url.is_some() {
        return Ok(selector);
    }

    let resolved =
        resolve_saved_hardware_selection(selector.device.clone(), default_devd, allow_interactive)?;
    match resolved {
        ResolvedHardware::Usb(resolved) => Ok(ApiSelector {
            url: None,
            device: Some(resolved.hardware_id),
        }),
        ResolvedHardware::Http { url, .. } => Ok(ApiSelector {
            url: Some(url),
            device: None,
        }),
    }
}

pub(crate) async fn request_http_value(
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

pub(crate) async fn request_devd_usb_value(
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

pub(crate) fn saved_usb_device_needs_relookup(
    error: &(dyn std::error::Error + Send + Sync),
) -> bool {
    let message = error.to_string();
    message.contains("device_not_found") || message.contains("identity_confirmation_mismatch")
}

pub(crate) fn resolve_scanned_usb_device_for_saved_hardware(
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

pub(crate) fn ensure_one_api_selector(
    url: Option<&String>,
    device: Option<&String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let count = [url.is_some(), device.is_some()]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    match count {
        0 => Ok(()),
        1 => Ok(()),
        _ => Err("command accepts only one of --device or --url".into()),
    }
}

pub(crate) fn ensure_one_status_selector(
    url: Option<&String>,
    device: Option<&String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let count = [url.is_some(), device.is_some()]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    match count {
        0 => Ok(()),
        1 => Ok(()),
        _ => Err("status accepts only one of --device or --url".into()),
    }
}

pub(crate) fn resolve_output_enable(
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
    allow_legacy_preflash_identity_fallback: bool,
) -> Result<CliLease, Box<dyn std::error::Error + Send + Sync>> {
    let mut body = serde_json::Map::new();
    body.insert("device_id".to_string(), json!(device));
    if let Some(expected) = expected_identity_device_id {
        body.insert("expected_identity_device_id".to_string(), json!(expected));
    }
    if allow_legacy_preflash_identity_fallback {
        body.insert(
            "allow_legacy_preflash_identity_fallback".to_string(),
            json!(true),
        );
    }
    Ok(serde_json::from_value(
        request_devd_value(
            devd,
            reqwest::Method::POST,
            "/api/v1/serial/lease",
            Some(Value::Object(body)),
        )
        .await?,
    )?)
}

pub(crate) async fn create_cli_lease_for_resolved_usb(
    client: &Client,
    resolved: &ResolvedUsbHardware,
) -> Result<(CliLease, String), Box<dyn std::error::Error + Send + Sync>> {
    create_cli_lease_for_resolved_usb_with_options(client, resolved, false).await
}

async fn create_cli_lease_for_resolved_usb_with_options(
    client: &Client,
    resolved: &ResolvedUsbHardware,
    allow_legacy_preflash_identity_fallback: bool,
) -> Result<(CliLease, String), Box<dyn std::error::Error + Send + Sync>> {
    match create_cli_lease_with_expected(
        client,
        &resolved.devd,
        &resolved.device,
        resolved.expected_identity_device_id.as_deref(),
        allow_legacy_preflash_identity_fallback,
    )
    .await
    {
        Ok(lease) => {
            if !allow_legacy_preflash_identity_fallback {
                validate_cli_lease_identity(&lease, resolved)?;
            }
            Ok((lease, resolved.device.clone()))
        }
        Err(error) if saved_usb_device_needs_relookup(&*error) => {
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
                resolved.expected_identity_device_id.as_deref(),
                allow_legacy_preflash_identity_fallback,
            )
            .await?;
            if !allow_legacy_preflash_identity_fallback {
                validate_cli_lease_identity(&lease, resolved)?;
            }
            Ok((lease, device))
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn validate_cli_lease_identity(
    lease: &CliLease,
    resolved: &ResolvedUsbHardware,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(expected) = resolved.expected_identity_device_id.as_deref() {
        match lease.identity_device_id.as_deref() {
            Some(actual) if actual == expected => {}
            Some(actual) => {
                return Err(format!(
                    "expected identity device_id {expected}, current identity is {actual}"
                )
                .into());
            }
            None => {
                return Err(format!(
                    "expected identity device_id {expected}, current identity is <missing>"
                )
                .into());
            }
        }
    }
    if resolved.expected_identity_device_id.is_none()
        && let Some(actual) = lease.identity_device_id.as_deref()
        && !is_stable_hardware_id(actual)
    {
        return Err(format!(
            "identity device_id `{actual}` is not a stable LoadLynx hardware id; update firmware before binding or controlling this device"
        )
        .into());
    }
    Ok(())
}

pub(crate) async fn create_cli_bind_probe_lease(
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

pub(crate) async fn release_cli_lease(
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

pub(crate) fn spawn_cli_lease_heartbeat(
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

pub(crate) async fn post_usb_operation_with_optional_lease(
    client: &Client,
    resolved: &ResolvedUsbHardware,
    path: &str,
    mut payload: Value,
    dry_run: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let lease = if dry_run {
        None
    } else {
        let allow_legacy_preflash_identity_fallback =
            path.ends_with("/flash") && resolved.expected_identity_device_id.is_some();
        Some(
            create_cli_lease_for_resolved_usb_with_options(
                client,
                resolved,
                allow_legacy_preflash_identity_fallback,
            )
            .await?,
        )
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

pub(crate) async fn run_monitor(
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
