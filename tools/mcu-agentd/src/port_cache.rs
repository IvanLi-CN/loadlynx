use crate::model::McuKind;
use crate::paths::Paths;
use anyhow::{Context, Result};
use std::fs;

pub fn read_port(paths: &Paths, mcu: McuKind) -> Result<Option<String>> {
    let p = match mcu {
        McuKind::Digital => &paths.esp32_port,
        McuKind::Analog => &paths.stm32_legacy,
    };
    if p.exists() {
        return Ok(Some(fs::read_to_string(p)?.trim().to_string()));
    }
    // legacy alias for analog
    if mcu == McuKind::Analog && paths.stm32_probe.exists() {
        return Ok(Some(
            fs::read_to_string(&paths.stm32_probe)?.trim().to_string(),
        ));
    }
    Ok(None)
}

pub fn write_port(paths: &Paths, mcu: McuKind, val: &str) -> Result<()> {
    let p = match mcu {
        McuKind::Digital => &paths.esp32_port,
        McuKind::Analog => &paths.stm32_legacy,
    };
    fs::write(p, val.trim()).with_context(|| format!("write {:?}", p))?;
    if mcu == McuKind::Analog {
        let _ = fs::write(&paths.stm32_probe, val.trim());
    }
    Ok(())
}
