use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_tail")]
    pub tail_default: usize,
    #[serde(default = "Config::default_heartbeat_secs")]
    pub heartbeat_secs: u64,
    #[serde(default = "Config::default_espflash_args")]
    pub espflash_args: String,
}

impl Config {
    fn default_tail() -> usize {
        200
    }
    fn default_heartbeat_secs() -> u64 {
        60
    }
    fn default_espflash_args() -> String {
        "--ignore_app_descriptor --non-interactive --skip-update-check".to_string()
    }
    pub fn load(repo_root: &PathBuf) -> Result<Self> {
        let path = repo_root.join("configs/mcu-agentd.toml");
        if path.exists() {
            let txt = fs::read_to_string(path)?;
            let mut cfg: Config = toml::from_str(&txt)?;
            if cfg.tail_default == 0 {
                cfg.tail_default = Self::default_tail();
            }
            if cfg.heartbeat_secs == 0 {
                cfg.heartbeat_secs = Self::default_heartbeat_secs();
            }
            if cfg.espflash_args.trim().is_empty() {
                cfg.espflash_args = Self::default_espflash_args();
            }
            Ok(cfg)
        } else {
            Ok(Config {
                tail_default: Self::default_tail(),
                heartbeat_secs: Self::default_heartbeat_secs(),
                espflash_args: Self::default_espflash_args(),
            })
        }
    }
}
