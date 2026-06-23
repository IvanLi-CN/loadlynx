use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub(crate) struct SerialProtocolFrame {
    pub(crate) direction: &'static str,
    pub(crate) frame: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct SerialProtocolProbe {
    pub(crate) frames: Vec<SerialProtocolFrame>,
    pub(crate) non_protocol_bytes: usize,
    pub(crate) non_protocol_text: String,
}

pub(crate) struct ExtractedSerialFrame {
    pub(crate) frame: Value,
    pub(crate) non_protocol_bytes: usize,
}

pub(crate) fn extract_serial_json_frames(line: &str) -> Vec<ExtractedSerialFrame> {
    if let Ok(frame) = serde_json::from_str::<Value>(line) {
        return vec![ExtractedSerialFrame {
            frame,
            non_protocol_bytes: 0,
        }];
    }

    let mut frames = Vec::new();
    let mut search_offset = 0;
    while let Some(start_rel) = line[search_offset..].find('{') {
        let start = search_offset + start_rel;
        let mut end_search = start + 1;
        let mut matched = None;
        while let Some(end_rel) = line[end_search..].find('}') {
            let end = end_search + end_rel + 1;
            if let Ok(frame) = serde_json::from_str::<Value>(&line[start..end]) {
                matched = Some((end, frame));
            }
            end_search = end;
        }

        match matched {
            Some((end, frame)) => {
                frames.push(ExtractedSerialFrame {
                    frame,
                    non_protocol_bytes: start.saturating_sub(search_offset),
                });
                search_offset = end;
            }
            None => {
                search_offset = start + 1;
            }
        }
    }

    if let Some(last) = frames.last_mut() {
        last.non_protocol_bytes += line.len().saturating_sub(search_offset);
    }
    frames
}

pub(crate) fn serial_response_for_request(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> Option<Value> {
    probe
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
}

pub(crate) fn serial_probe_has_mismatched_response(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> bool {
    probe.frames.iter().any(|event| {
        event.direction == "rx"
            && event
                .frame
                .get("request_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id != request_id)
    })
}

pub(crate) fn serial_request_id_matches_op(request_id: &str, legacy_id: &str) -> bool {
    request_id == legacy_id
        || request_id
            .strip_prefix(legacy_id)
            .is_some_and(|suffix| suffix.starts_with('-'))
}

pub(crate) fn infer_serial_response_from_text(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> Option<Value> {
    let text = &probe.non_protocol_text;
    if serial_request_id_matches_op(request_id, "devd-get-calibration-profile")
        && let Some(response) = infer_calibration_profile_response_from_text(text, request_id)
    {
        return Some(response);
    }
    if serial_request_id_matches_op(request_id, "devd-get-wifi-status")
        && let Some(response) = infer_wifi_status_response_from_text(text, request_id)
    {
        return Some(response);
    }
    if is_output_control_request_id(request_id)
        && text.contains(request_id)
        && text.contains("\"ok\":true")
    {
        let requested_enable = probe.frames.iter().find_map(|event| {
            (event.direction == "tx"
                && event
                    .frame
                    .get("request_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == request_id))
            .then(|| event.frame.get("enable").and_then(Value::as_bool))
            .flatten()
        });
        let output_enabled = if text.contains("\"output_enabled\":false") {
            Some(false)
        } else if text.contains("\"output_enabled\":true") {
            Some(true)
        } else {
            requested_enable
        };
        let changed = if text.contains("\"changed\":false") {
            Some(false)
        } else if text.contains("\"changed\":true") {
            Some(true)
        } else {
            None
        };
        let mut data = serde_json::Map::new();
        if let Some(output_enabled) = output_enabled {
            data.insert("output_enabled".to_string(), json!(output_enabled));
        }
        if let Some(changed) = changed {
            data.insert("changed".to_string(), json!(changed));
        }
        return Some(json!({
            "type": "response",
            "request_id": request_id,
            "ok": true,
            "data": Value::Object(data),
            "recovered_from_text": true
        }));
    }
    None
}

fn infer_calibration_profile_response_from_text(text: &str, request_id: &str) -> Option<Value> {
    if !text.contains("\"a\":[")
        || !text.contains("\"c1\":")
        || !text.contains("\"c2\":")
        || !text.contains("\"vl\":")
        || !text.contains("\"vr\":")
    {
        return None;
    }
    let start = text.find("\"a\":[")?;
    let vr_key = text[start..].find("\"vr\":")? + start;
    let vr_array = text[vr_key..].find('[')? + vr_key;
    let end = json_array_end(text, vr_array)?;
    let candidate = format!("{{\"compact\":\"cal_profile_v1\",{}}}", &text[start..end]);
    let data = serde_json::from_str::<Value>(&candidate).ok()?;
    Some(json!({
        "type": "response",
        "request_id": request_id,
        "ok": true,
        "data": data,
        "recovered_from_text": true
    }))
}

fn infer_wifi_status_response_from_text(text: &str, request_id: &str) -> Option<Value> {
    let state = json_string_after_any(text, &["\"state\":\"", "tate\":\""])?;
    let ip = json_string_after_any(text, &["\"ip\":\""]);
    let ssid = json_string_after_any(text, &["\"ssid\":\""]);
    let source = json_string_after_any(text, &["\"source\":\""]);
    let last_error = json_string_after_any(text, &["\"last_error\":\""]);
    let data = json!({
        "ssid": ssid,
        "source": source,
        "state": state,
        "ip": ip,
        "last_error": last_error,
        "recovered_from_text": true
    });
    Some(json!({
        "type": "response",
        "request_id": request_id,
        "ok": true,
        "data": data,
        "recovered_from_text": true
    }))
}

fn json_string_after_any(text: &str, markers: &[&str]) -> Option<String> {
    for marker in markers {
        if let Some(value) = json_string_after(text, marker) {
            return Some(value);
        }
    }
    None
}

fn json_string_after(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)? + marker.len();
    let mut out = String::new();
    let mut escaped = false;
    for ch in text[start..].chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn json_array_end(text: &str, array_start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in text[array_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(array_start + offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn infer_serial_response_from_fragments(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> Option<Value> {
    if serial_request_id_matches_op(request_id, "devd-get-identity") {
        return infer_identity_response_from_fragments(probe, request_id);
    }
    if serial_request_id_matches_op(request_id, "devd-get-control") {
        return infer_control_response_from_fragments(probe, request_id);
    }
    if serial_request_id_matches_op(request_id, "devd-get-presets") {
        return infer_presets_response_from_fragments(probe, request_id);
    }
    if !serial_request_id_matches_op(request_id, "devd-get-status") {
        return None;
    }
    let tx_index = probe.frames.iter().position(|event| {
        event.direction == "tx"
            && event
                .frame
                .get("request_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == request_id)
    })?;
    let mut control = None;
    let mut status = None;
    for event in probe.frames.iter().skip(tx_index + 1) {
        if event.direction != "rx" || event.frame.get("request_id").is_some() {
            continue;
        }
        let frame = &event.frame;
        if frame.get("status").is_some() {
            let mut data = frame.clone();
            data.as_object_mut()?
                .insert("recovered_from_fragments".to_string(), json!(true));
            return Some(json!({
                "type": "response",
                "request_id": request_id,
                "ok": true,
                "data": data,
                "recovered_from_fragments": true
            }));
        }
        if frame.get("active_preset_id").is_some()
            && frame.get("mode").is_some()
            && frame.get("output_enabled").is_some()
        {
            control = Some(frame.clone());
        }
        if frame.get("state_flags").is_some()
            && frame.get("fault_flags").is_some()
            && frame.get("v_local_mv").is_some()
            && frame.get("i_local_ma").is_some()
        {
            status = Some(frame.clone());
        }
        if control.is_some() && status.is_some() {
            break;
        }
    }
    let status = status?;
    let mut data = serde_json::Map::new();
    data.insert("status".to_string(), status);
    if let Some(control) = control {
        data.insert("control".to_string(), control);
    }
    data.insert("link_up".to_string(), json!(true));
    data.insert("hello_seen".to_string(), json!(true));
    data.insert("recovered_from_fragments".to_string(), json!(true));
    Some(json!({
        "type": "response",
        "request_id": request_id,
        "ok": true,
        "data": Value::Object(data),
        "recovered_from_fragments": true
    }))
}

pub(crate) fn probe_has_recoverable_response(
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

fn is_stable_hardware_id(id: &str) -> bool {
    id.strip_prefix("loadlynx-").is_some_and(is_hex_short_id) || id.starts_with("mock-")
}

fn is_hex_short_id(value: &str) -> bool {
    value.len() == 6
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub(crate) fn is_preset_fragment(frame: &Value) -> bool {
    frame.get("preset_id").is_some()
        && frame.get("mode").is_some()
        && frame.get("max_i_ma_total").is_some()
        && frame.get("max_p_mw").is_some()
}

fn infer_control_response_from_fragments(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> Option<Value> {
    let tx_index = probe.frames.iter().position(|event| {
        event.direction == "tx"
            && event
                .frame
                .get("request_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == request_id)
    })?;
    for event in probe.frames.iter().skip(tx_index + 1) {
        if event.direction != "rx" || event.frame.get("request_id").is_some() {
            continue;
        }
        let frame = &event.frame;
        if frame.get("active_preset_id").is_some()
            && frame.get("preset").is_some()
            && frame.get("output_enabled").is_some()
        {
            let mut data = frame.clone();
            data.as_object_mut()?
                .insert("recovered_from_fragments".to_string(), json!(true));
            return Some(json!({
                "type": "response",
                "request_id": request_id,
                "ok": true,
                "data": data,
                "recovered_from_fragments": true
            }));
        }
        if is_preset_fragment(frame) {
            let active_preset_id = frame.get("preset_id").cloned().unwrap_or(json!(1));
            return Some(json!({
                "type": "response",
                "request_id": request_id,
                "ok": true,
                "data": {
                    "active_preset_id": active_preset_id,
                    "output_enabled": false,
                    "uv_latched": false,
                    "preset": frame,
                    "recovered_from_fragments": true
                },
                "recovered_from_fragments": true
            }));
        }
    }
    None
}

fn infer_presets_response_from_fragments(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> Option<Value> {
    let tx_index = probe.frames.iter().position(|event| {
        event.direction == "tx"
            && event
                .frame
                .get("request_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == request_id)
    })?;
    let mut presets = probe
        .frames
        .iter()
        .skip(tx_index + 1)
        .filter_map(|event| {
            if event.direction == "rx"
                && event.frame.get("request_id").is_none()
                && is_preset_fragment(&event.frame)
            {
                Some(event.frame.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if presets.is_empty() {
        return None;
    }
    presets.sort_by_key(|preset| {
        preset
            .get("preset_id")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX)
    });
    Some(json!({
        "type": "response",
        "request_id": request_id,
        "ok": true,
        "data": {
            "presets": presets,
            "recovered_from_fragments": true
        },
        "recovered_from_fragments": true
    }))
}

fn infer_identity_response_from_fragments(
    probe: &SerialProtocolProbe,
    request_id: &str,
) -> Option<Value> {
    let tx_index = probe.frames.iter().position(|event| {
        event.direction == "tx"
            && event
                .frame
                .get("request_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == request_id)
    })?;
    let mut firmware_frame = None;
    let mut stable_identity = None;
    for event in probe.frames.iter().skip(tx_index + 1) {
        if event.direction != "rx" || event.frame.get("request_id").is_some() {
            continue;
        }
        let frame = &event.frame;
        if stable_identity.is_none()
            && let Some(id) = frame.get("device_id").and_then(Value::as_str)
            && is_stable_hardware_id(id)
        {
            stable_identity = Some(frame.clone());
        }
        if firmware_frame.is_none()
            && frame.get("build_id").is_some()
            && frame.get("target").and_then(Value::as_str) == Some("digital_esp32s3")
        {
            firmware_frame = Some(frame.clone());
        }
        if stable_identity.is_some() && firmware_frame.is_some() {
            break;
        }
    }
    let stable_identity = stable_identity?;
    let firmware = firmware_frame.unwrap_or_else(|| json!({"target": "digital_esp32s3"}));
    let device_id = stable_identity
        .get("device_id")
        .and_then(Value::as_str)
        .filter(|id| is_stable_hardware_id(id))?;
    let build_id = firmware
        .get("build_id")
        .and_then(Value::as_str)
        .unwrap_or("digital unknown");
    let protocol = firmware
        .get("protocol")
        .and_then(Value::as_str)
        .unwrap_or("loadlynx.cdc.v1");
    Some(json!({
        "type": "response",
        "request_id": request_id,
        "ok": true,
        "data": {
            "device_id": device_id,
            "target": "digital",
            "mcu": "esp32s3",
            "protocol": protocol,
            "firmware_version": build_id,
            "digital_fw_version": build_id,
            "firmware": firmware,
            "recovered_from_fragments": true,
            "stable_identity": stable_identity
        },
        "recovered_from_fragments": true
    }))
}

fn is_output_control_request_id(request_id: &str) -> bool {
    request_id.starts_with("devd-output-")
        || serial_request_id_matches_op(request_id, "devd-set-output-enabled")
}

pub(crate) fn sanitize_trace_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_control() {
            use std::fmt::Write as _;
            let _ = write!(out, "\\x{:02x}", ch as u32);
        } else {
            out.push(ch);
        }
    }
    out
}
