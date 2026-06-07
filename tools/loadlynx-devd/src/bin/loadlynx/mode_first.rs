use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliPreset {
    preset_id: u8,
    mode: String,
    target_i_ma: u32,
    target_v_mv: u32,
    target_p_mw: u32,
    min_v_mv: u32,
    max_i_ma_total: u32,
    max_p_mw: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliControlView {
    active_preset_id: u8,
    output_enabled: bool,
    uv_latched: bool,
    preset: CliPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliPresetsEnvelope {
    presets: Vec<CliPreset>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModeFirstCommand {
    Cc,
    Cv,
    Cp,
}

impl ModeFirstCommand {
    fn mode(&self) -> &'static str {
        match self {
            Self::Cc => "cc",
            Self::Cv => "cv",
            Self::Cp => "cp",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Cc => "CC",
            Self::Cv => "CV",
            Self::Cp => "CP",
        }
    }
}

fn build_cli_preset(
    control: &CliControlView,
    presets: &[CliPreset],
    preset_id: Option<u8>,
) -> Result<CliPreset, Box<dyn std::error::Error + Send + Sync>> {
    let selected_id = preset_id.unwrap_or(control.active_preset_id);
    presets
        .iter()
        .find(|preset| preset.preset_id == selected_id)
        .cloned()
        .ok_or_else(|| format!("preset {selected_id} not found").into())
}

pub(crate) fn validate_mode_first_targets(
    mode: ModeFirstCommand,
    target_i_ma: u32,
    target_v_mv: Option<u32>,
    target_p_mw: Option<u32>,
    max_i_ma_total: u32,
    max_p_mw: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match mode {
        ModeFirstCommand::Cc => {
            if target_i_ma > max_i_ma_total {
                return Err(format!(
                    "target_i_ma exceeds max_i_ma_total ({} > {})",
                    target_i_ma, max_i_ma_total
                )
                .into());
            }
        }
        ModeFirstCommand::Cv => {
            target_v_mv.ok_or("target_v_mv is required for cv")?;
        }
        ModeFirstCommand::Cp => {
            let target_p_mw = target_p_mw.ok_or("target_p_mw is required for cp")?;
            if target_p_mw > max_p_mw {
                return Err(format!(
                    "target_p_mw exceeds max_p_mw ({} > {})",
                    target_p_mw, max_p_mw
                )
                .into());
            }
        }
    }
    Ok(())
}

fn legacy_cc_output_enabled(response: &Value) -> Option<bool> {
    response
        .get("output_enabled")
        .and_then(Value::as_bool)
        .or_else(|| response.get("enable").and_then(Value::as_bool))
        .or_else(|| {
            response
                .pointer("/response/output_enabled")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            response
                .pointer("/response/enable")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            response
                .pointer("/response/data/output_enabled")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            response
                .pointer("/response/data/enable")
                .and_then(Value::as_bool)
        })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_mode_first_command(
    client: &Client,
    default_devd: &str,
    mode: ModeFirstCommand,
    target_i_ma: u32,
    target_v_mv: Option<u32>,
    target_p_mw: Option<u32>,
    url: Option<String>,
    hardware: Option<String>,
    preset_id: Option<u8>,
    min_v_mv: Option<u32>,
    max_i_ma_total: Option<u32>,
    max_p_mw: Option<u32>,
    disable: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let selector = ApiSelector {
        url,
        device: None,
        hardware,
    };
    handle_mode_first_command_for_selector(
        client,
        default_devd,
        selector,
        mode,
        target_i_ma,
        target_v_mv,
        target_p_mw,
        preset_id,
        min_v_mv,
        max_i_ma_total,
        max_p_mw,
        disable,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_mode_first_command_for_selector(
    client: &Client,
    default_devd: &str,
    selector: ApiSelector,
    mode: ModeFirstCommand,
    target_i_ma: u32,
    target_v_mv: Option<u32>,
    target_p_mw: Option<u32>,
    preset_id: Option<u8>,
    min_v_mv: Option<u32>,
    max_i_ma_total: Option<u32>,
    max_p_mw: Option<u32>,
    disable: bool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let identity = request_api_value(
        client,
        default_devd,
        selector.clone(),
        reqwest::Method::GET,
        "/api/v1/identity",
        None,
        false,
    )
    .await?;
    let presets_supported = identity
        .get("capabilities")
        .and_then(|capabilities| capabilities.get("presets_supported"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if disable && !presets_supported {
        let mut body = serde_json::Map::new();
        body.insert("enable".to_string(), Value::Bool(false));

        let cc = request_api_value(
            client,
            default_devd,
            selector,
            reqwest::Method::POST,
            "/api/v1/cc",
            Some(Value::Object(body)),
            false,
        )
        .await?;
        let output_enabled = legacy_cc_output_enabled(&cc).unwrap_or(false);

        return Ok(serde_json::json!({
            "mode": mode.label(),
            "output_enabled": output_enabled,
            "cc": cc,
        }));
    }

    if disable {
        let control = serde_json::from_value::<CliControlView>(
            request_api_value(
                client,
                default_devd,
                selector,
                reqwest::Method::POST,
                "/api/v1/control",
                Some(json!({"output_enabled": false})),
                false,
            )
            .await?,
        )?;

        return Ok(serde_json::json!({
            "mode": mode.label(),
            "preset_id": control.active_preset_id,
            "output_enabled": control.output_enabled,
            "preset": control.preset,
        }));
    }

    if matches!(mode, ModeFirstCommand::Cc) && !presets_supported {
        let mut body = serde_json::Map::new();
        body.insert("enable".to_string(), Value::Bool(true));
        body.insert("target_i_ma".to_string(), json!(target_i_ma));

        let cc = request_api_value(
            client,
            default_devd,
            selector,
            reqwest::Method::POST,
            "/api/v1/cc",
            Some(Value::Object(body)),
            false,
        )
        .await?;
        let output_enabled = legacy_cc_output_enabled(&cc).unwrap_or(true);

        return Ok(serde_json::json!({
            "mode": mode.label(),
            "target_i_ma": target_i_ma,
            "output_enabled": output_enabled,
            "cc": cc,
        }));
    }

    if !presets_supported {
        return Err("preset APIs are required for cv/cp on this device".into());
    }

    let control = serde_json::from_value::<CliControlView>(
        request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::GET,
            "/api/v1/control",
            None,
            false,
        )
        .await?,
    )?;
    let presets = serde_json::from_value::<CliPresetsEnvelope>(
        request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::GET,
            "/api/v1/presets",
            None,
            false,
        )
        .await?,
    )?;
    let mut preset = build_cli_preset(&control, &presets.presets, preset_id)?;

    match mode {
        ModeFirstCommand::Cc => {
            let max_i_ma_total = max_i_ma_total.unwrap_or(preset.max_i_ma_total);
            validate_mode_first_targets(
                mode,
                target_i_ma,
                target_v_mv,
                target_p_mw,
                max_i_ma_total,
                preset.max_p_mw,
            )?;
            preset.mode = mode.mode().to_string();
            preset.target_i_ma = target_i_ma;
            if let Some(min_v_mv) = min_v_mv {
                preset.min_v_mv = min_v_mv;
            }
            preset.max_i_ma_total = max_i_ma_total;
            if let Some(max_p_mw) = max_p_mw {
                preset.max_p_mw = max_p_mw;
            }
        }
        ModeFirstCommand::Cv => {
            let target_v_mv = target_v_mv.ok_or("target_v_mv is required for cv")?;
            validate_mode_first_targets(
                mode,
                target_i_ma,
                Some(target_v_mv),
                target_p_mw,
                max_i_ma_total.unwrap_or(preset.max_i_ma_total),
                max_p_mw.unwrap_or(preset.max_p_mw),
            )?;
            preset.mode = mode.mode().to_string();
            preset.target_v_mv = target_v_mv;
            if let Some(min_v_mv) = min_v_mv {
                preset.min_v_mv = min_v_mv;
            }
            if let Some(max_i_ma_total) = max_i_ma_total {
                preset.max_i_ma_total = max_i_ma_total;
            }
            if let Some(max_p_mw) = max_p_mw {
                preset.max_p_mw = max_p_mw;
            }
        }
        ModeFirstCommand::Cp => {
            let target_p_mw = target_p_mw.ok_or("target_p_mw is required for cp")?;
            let max_p_mw = max_p_mw.unwrap_or(preset.max_p_mw);
            validate_mode_first_targets(
                mode,
                target_i_ma,
                target_v_mv,
                Some(target_p_mw),
                max_i_ma_total.unwrap_or(preset.max_i_ma_total),
                max_p_mw,
            )?;
            preset.mode = mode.mode().to_string();
            preset.target_p_mw = target_p_mw;
            if let Some(min_v_mv) = min_v_mv {
                preset.min_v_mv = min_v_mv;
            }
            if let Some(max_i_ma_total) = max_i_ma_total {
                preset.max_i_ma_total = max_i_ma_total;
            }
            preset.max_p_mw = max_p_mw;
        }
    }

    request_api_value(
        client,
        default_devd,
        selector.clone(),
        reqwest::Method::POST,
        "/api/v1/presets",
        Some(serde_json::to_value(&preset)?),
        false,
    )
    .await?;

    let mut control = serde_json::from_value::<CliControlView>(
        request_api_value(
            client,
            default_devd,
            selector.clone(),
            reqwest::Method::POST,
            "/api/v1/presets/apply",
            Some(json!({"preset_id": preset.preset_id})),
            false,
        )
        .await?,
    )?;

    if !disable {
        control = serde_json::from_value::<CliControlView>(
            request_api_value(
                client,
                default_devd,
                selector,
                reqwest::Method::POST,
                "/api/v1/control",
                Some(json!({"output_enabled": true})),
                false,
            )
            .await?,
        )?;
    }

    Ok(serde_json::json!({
        "mode": mode.label(),
        "preset_id": control.active_preset_id,
        "output_enabled": control.output_enabled,
        "preset": control.preset,
    }))
}
