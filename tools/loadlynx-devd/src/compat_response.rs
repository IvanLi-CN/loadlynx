use crate::HttpError;
use crate::serial_response::is_preset_fragment;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(crate) fn serial_response_data_required(
    response: Option<Value>,
    operation: &str,
) -> Result<Value, HttpError> {
    let response = response.ok_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            format!("{operation} did not return a protocol response"),
        )
    })?;
    serial_response_data(response, operation)
}

pub(crate) fn merge_presets_from_data(merged: &mut HashMap<u64, Value>, data: &Value) -> bool {
    if is_preset_fragment(data) {
        return merge_preset_value(merged, data);
    }
    if let Some(preset) = data.get("preset")
        && is_preset_fragment(preset)
    {
        return merge_preset_value(merged, preset);
    }
    let Some(presets) = data.get("presets").and_then(Value::as_array) else {
        return false;
    };
    let mut merged_any = false;
    for preset in presets {
        merged_any |= merge_preset_value(merged, preset);
    }
    merged_any
}

pub(crate) fn presets_data_from_map(
    merged: &HashMap<u64, Value>,
    recovered: bool,
    recovered_by_control: bool,
) -> Value {
    let mut presets = merged
        .iter()
        .map(|(id, preset)| (*id, preset.clone()))
        .collect::<Vec<_>>();
    presets.sort_by_key(|(id, _)| *id);
    let mut data = json!({
        "presets": presets
            .into_iter()
            .map(|(_, preset)| preset)
            .collect::<Vec<_>>()
    });
    if recovered && let Some(object) = data.as_object_mut() {
        object.insert("recovered_from_fragments".to_string(), json!(true));
        object.insert("recovered_by_retry".to_string(), json!(true));
    }
    if recovered_by_control && let Some(object) = data.as_object_mut() {
        object.insert("recovered_from_fragments".to_string(), json!(true));
        object.insert("recovered_by_control".to_string(), json!(true));
    }
    data
}

pub(crate) fn pd_response_data(response: Value) -> Result<Value, HttpError> {
    serial_response_data(response, "USB PD")
}

pub(crate) fn pd_post_response_data(response: Option<Value>) -> Result<Value, HttpError> {
    response.map(pd_response_data).unwrap_or_else(|| {
        Err(HttpError::retryable(
            "serial_response_missing",
            "USB PD POST did not return a protocol response",
        ))
    })
}

pub(crate) fn identity_data_from_serial_response(
    response: Option<Value>,
) -> Result<Value, HttpError> {
    let response = response.ok_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            "USB identity did not return a protocol response",
        )
    })?;
    let data = serial_response_data(response, "USB identity")?;
    if data.get("device_id").is_none() {
        return Err(HttpError::retryable(
            "serial_response_invalid",
            "USB identity response did not include device_id",
        ));
    }
    Ok(data)
}

pub(crate) fn status_data_from_serial_response(
    response: Option<Value>,
) -> Result<Value, HttpError> {
    let response = response.ok_or_else(|| {
        HttpError::retryable(
            "serial_response_missing",
            "USB status did not return a protocol response",
        )
    })?;
    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        return serial_response_data(response, "USB status");
    }
    let data = response
        .get("data")
        .cloned()
        .unwrap_or_else(|| response.clone());
    if data.get("status").is_none() {
        return Err(HttpError::retryable(
            "serial_response_invalid",
            "USB status response did not include status",
        ));
    }
    Ok(data)
}

pub(crate) fn expand_compact_calibration_profile(data: Value) -> Result<Value, HttpError> {
    let Some(compact) = data.as_object() else {
        return Ok(data);
    };
    if compact.get("compact").and_then(Value::as_str) != Some("cal_profile_v1") {
        return Ok(data);
    }

    let active = compact_array(compact, "a")?;
    if active.len() != 3 {
        return Err(compact_calibration_error("active tuple must have 3 items"));
    }
    let source = active[0]
        .as_str()
        .ok_or_else(|| compact_calibration_error("active source must be a string"))?;
    let fmt_version = compact_u64(&active[1], "fmt_version")?;
    let hw_rev = compact_u64(&active[2], "hw_rev")?;

    Ok(json!({
        "active": {
            "source": source,
            "fmt_version": fmt_version,
            "hw_rev": hw_rev,
        },
        "current_ch1_points": expand_compact_current_curve(compact, "c1")?,
        "current_ch2_points": expand_compact_current_curve(compact, "c2")?,
        "v_local_points": expand_compact_voltage_curve(compact, "vl")?,
        "v_remote_points": expand_compact_voltage_curve(compact, "vr")?,
    }))
}

pub(crate) fn serial_response_data(response: Value, operation: &str) -> Result<Value, HttpError> {
    if response.get("ok").and_then(Value::as_bool) == Some(true) {
        return response.get("data").cloned().ok_or_else(|| {
            HttpError::retryable(
                "serial_response_invalid",
                format!("{operation} response did not include data"),
            )
        });
    }

    let code = response
        .pointer("/error/code")
        .and_then(Value::as_str)
        .unwrap_or("serial_request_failed");
    let message = response
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("USB request failed");
    Err(HttpError::conflict(code, message))
}

fn merge_preset_value(merged: &mut HashMap<u64, Value>, preset: &Value) -> bool {
    let Some(id) = preset.get("preset_id").and_then(Value::as_u64) else {
        return false;
    };
    merged.insert(id, preset.clone());
    true
}

fn compact_array<'a>(
    compact: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, HttpError> {
    compact.get(key).and_then(Value::as_array).ok_or_else(|| {
        compact_calibration_error(format!("compact calibration field {key} missing"))
    })
}

fn expand_compact_current_curve(
    compact: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Value, HttpError> {
    let mut points = Vec::new();
    for point in compact_array(compact, key)? {
        let tuple = point
            .as_array()
            .ok_or_else(|| compact_calibration_error("current point must be an array"))?;
        if tuple.len() != 3 {
            return Err(compact_calibration_error(
                "current point tuple must have 3 items",
            ));
        }
        points.push(json!({
            "raw_100uv": compact_i64(&tuple[0], "raw_100uv")?,
            "raw_dac_code": compact_u64(&tuple[1], "raw_dac_code")?,
            "meas_ma": compact_i64(&tuple[2], "meas_ma")?,
        }));
    }
    Ok(Value::Array(points))
}

fn expand_compact_voltage_curve(
    compact: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Value, HttpError> {
    let mut points = Vec::new();
    for point in compact_array(compact, key)? {
        let tuple = point
            .as_array()
            .ok_or_else(|| compact_calibration_error("voltage point must be an array"))?;
        if tuple.len() != 2 {
            return Err(compact_calibration_error(
                "voltage point tuple must have 2 items",
            ));
        }
        points.push(json!({
            "raw_100uv": compact_i64(&tuple[0], "raw_100uv")?,
            "meas_mv": compact_i64(&tuple[1], "meas_mv")?,
        }));
    }
    Ok(Value::Array(points))
}

fn compact_i64(value: &Value, field: &str) -> Result<i64, HttpError> {
    value
        .as_i64()
        .ok_or_else(|| compact_calibration_error(format!("{field} must be an integer")))
}

fn compact_u64(value: &Value, field: &str) -> Result<u64, HttpError> {
    value
        .as_u64()
        .ok_or_else(|| compact_calibration_error(format!("{field} must be a non-negative integer")))
}

fn compact_calibration_error(message: impl Into<String>) -> HttpError {
    HttpError::retryable("serial_response_invalid", message)
}
