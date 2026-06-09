use super::*;

pub(crate) fn print_cli_payload(
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

pub(crate) fn print_cli_error(
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

pub(crate) fn classify_cli_error_code(message: &str) -> &'static str {
    if message.contains("default device is not set") {
        "default_device_not_set"
    } else if message.contains("saved device not found") {
        "device_not_found"
    } else if message.contains("is not a stable LoadLynx hardware id")
        || message.contains("is not a stable LoadLynx device id")
    {
        "unstable_hardware_identity"
    } else if message.contains("identity_confirmation_mismatch") {
        "identity_confirmation_mismatch"
    } else if message.contains("device_not_found") {
        "device_not_found"
    } else if message.contains("target_selector_not_cached") {
        "target_selector_not_cached"
    } else if message.contains("bind it first") || message.contains("run `loadlynx device add`") {
        "device_not_bound"
    } else {
        "command_failed"
    }
}

pub(crate) fn render_human_payload(payload: &Value) -> Result<String, serde_json::Error> {
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

    if let Some(mode) = str_field(payload, "mode")
        && payload.get("output_enabled").is_some()
    {
        let target = payload
            .get("target_i_ma")
            .and_then(Value::as_u64)
            .map(|target| format!(" target_i_ma={target}"))
            .unwrap_or_default();
        let preset = payload
            .get("preset_id")
            .and_then(Value::as_u64)
            .map(|preset| format!(" preset={preset}"))
            .unwrap_or_default();
        return Ok(format!(
            "{mode}: output={}{}{}",
            bool_field(payload, "output_enabled").unwrap_or(false),
            target,
            preset
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

    if let Some(devices) = payload.get("devices").and_then(Value::as_array)
        && payload.get("global_default_device_id").is_some()
    {
        return Ok(render_saved_devices_payload(payload, devices));
    }

    if payload.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok("OK".to_string());
    }

    serde_json::to_string_pretty(payload)
}

fn render_saved_devices_payload(payload: &Value, devices: &[Value]) -> String {
    let mut out = String::new();
    let global_default = str_field(payload, "global_default_device_id");
    if let Some(local) = payload.get("local_device") {
        let path = local
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(".loadlynx");
        let id = local
            .get("device_id")
            .and_then(Value::as_str)
            .unwrap_or("-");
        out.push_str(&format!("Local: {id} ({path})"));
    } else {
        out.push_str("Local: none");
    }
    out.push('\n');
    out.push_str(&format!("Global: {}", global_default.unwrap_or("none")));
    out.push('\n');
    out.push_str(&format!("Saved devices: {}", devices.len()));
    for device in devices {
        out.push_str("\n  ");
        out.push_str(&render_saved_device_line(device, global_default));
    }
    out
}

fn render_saved_device_line(device: &Value, global_default: Option<&str>) -> String {
    let id = device.get("id").and_then(Value::as_str).unwrap_or("-");
    let name = device.get("name").and_then(Value::as_str);
    let last_transport = device
        .get("last_transport")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let has_usb = device.pointer("/transports/usb").is_some();
    let has_http = device.pointer("/transports/http").is_some();

    let mut parts = vec![id.to_string()];
    if let Some(name) = name {
        parts.push(format!("name={name}"));
    }
    parts.push(format!("pref={last_transport}"));
    if has_usb {
        parts.push("usb".to_string());
    }
    if has_http {
        parts.push("http".to_string());
    }
    if global_default == Some(id) {
        parts.push("global-default".to_string());
    }
    parts.join(" ")
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
