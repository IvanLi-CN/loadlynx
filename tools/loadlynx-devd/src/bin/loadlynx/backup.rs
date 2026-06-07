use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct BackupSelection {
    pub(crate) presets: bool,
    pub(crate) calibration: bool,
    pub(crate) wifi: bool,
    pub(crate) pd: bool,
}

impl BackupSelection {
    pub(crate) fn all() -> Self {
        Self {
            presets: true,
            calibration: true,
            wifi: true,
            pd: true,
        }
    }

    pub(crate) fn selected_names(&self) -> Vec<&'static str> {
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

pub(crate) fn parse_backup_selection(
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

pub(crate) async fn handle_backup_export(
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

pub(crate) async fn handle_backup_import(
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

pub(crate) fn preflight_backup_restore(
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

pub(crate) fn write_backup_file(
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

pub(crate) fn validate_backup_envelope(
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

pub(crate) fn backup_unknown_section_warnings(backup: &Value) -> Vec<Value> {
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

pub(crate) fn restorable_backup_sections(
    backup: &Value,
    selection: BackupSelection,
) -> Vec<&'static str> {
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

pub(crate) fn calibration_curve_write_body(
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
    if let Some(saved) = pd.get("saved").and_then(Value::as_object)
        && let Some(mode) = saved.get("mode").and_then(Value::as_str)
    {
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
        if let Some(i_req_ma) = saved.get("i_req_ma").cloned() {
            body.insert("i_req_ma".to_string(), i_req_ma);
        }
    }
    if let Some(allow_extended_voltage) = pd.get("allow_extended_voltage").cloned() {
        body.insert("allow_extended_voltage".to_string(), allow_extended_voltage);
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
