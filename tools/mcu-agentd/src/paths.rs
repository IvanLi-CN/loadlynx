use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub sock: PathBuf,
    pub lock: PathBuf,
    pub logs_dir: PathBuf,
    pub tmp_logs_dir: PathBuf,
    pub meta_digital: PathBuf,
    pub meta_analog: PathBuf,
    pub session_digital: PathBuf,
    pub session_analog: PathBuf,
    pub mon_digital: PathBuf,
    pub mon_analog: PathBuf,
    pub esp32_port: PathBuf,
    pub stm32_port: PathBuf,
    pub analog_fw_version: PathBuf,
    pub analog_last_flashed: PathBuf,
    pub digital_fw_version: PathBuf,
    pub digital_last_flashed: PathBuf,
}

impl Paths {
    pub fn new() -> Result<Self> {
        let root = find_repo_root()?;
        let logs_dir = root.join("logs/agentd");
        let tmp_logs_dir = root.join("tmp/agent-logs");
        let sock = logs_dir.join("agentd.sock");
        let lock = logs_dir.join("agentd.lock");
        let meta_digital = logs_dir.join("digital.meta.log");
        let meta_analog = logs_dir.join("analog.meta.log");
        let session_digital = logs_dir.join("digital");
        let session_analog = logs_dir.join("analog");
        let mon_digital = logs_dir.join("digital/monitor");
        let mon_analog = logs_dir.join("analog/monitor");
        let esp32_port = root.join(".esp32-port");
        let stm32_port = root.join(".stm32-port");
        let analog_fw_version = root.join("tmp/analog-fw-version.txt");
        let analog_last_flashed = root.join("tmp/analog-fw-last-flashed.txt");
        let digital_fw_version = root.join("tmp/digital-fw-version.txt");
        let digital_last_flashed = root.join("tmp/digital-fw-last-flashed.txt");
        Ok(Self {
            root,
            sock,
            lock,
            logs_dir,
            tmp_logs_dir,
            meta_digital,
            meta_analog,
            session_digital,
            session_analog,
            mon_digital,
            mon_analog,
            esp32_port,
            stm32_port,
            analog_fw_version,
            analog_last_flashed,
            digital_fw_version,
            digital_last_flashed,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.logs_dir)?;
        std::fs::create_dir_all(&self.session_digital)?;
        std::fs::create_dir_all(&self.session_analog)?;
        std::fs::create_dir_all(self.tmp_logs_parent())?;
        std::fs::create_dir_all(&self.tmp_logs_dir)?;
        std::fs::create_dir_all(&self.mon_digital)?;
        std::fs::create_dir_all(&self.mon_analog)?;
        std::fs::create_dir_all(self.sock.parent().unwrap())?;
        Ok(())
    }

    pub fn meta(&self, mcu: crate::model::McuKind) -> &Path {
        match mcu {
            crate::model::McuKind::Digital => self.meta_digital.as_path(),
            crate::model::McuKind::Analog => self.meta_analog.as_path(),
        }
    }

    pub fn session_dir(&self, mcu: crate::model::McuKind) -> &Path {
        match mcu {
            crate::model::McuKind::Digital => self.session_digital.as_path(),
            crate::model::McuKind::Analog => self.session_analog.as_path(),
        }
    }

    pub fn monitor_dir(&self, mcu: crate::model::McuKind) -> &Path {
        match mcu {
            crate::model::McuKind::Digital => self.mon_digital.as_path(),
            crate::model::McuKind::Analog => self.mon_analog.as_path(),
        }
    }

    pub fn lock_path(&self) -> &Path {
        self.lock.as_path()
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn tmp_logs_parent(&self) -> &Path {
        self.tmp_logs_dir
            .parent()
            .unwrap_or_else(|| Path::new("tmp"))
    }

    pub fn log_file(&self) -> PathBuf {
        self.logs_dir.join("agentd.log")
    }
}

fn find_repo_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let marker = dir.join("loadlynx.ioc");
        let scripts = dir.join("scripts/agent_verify_analog.sh");
        if marker.exists() || scripts.exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!("unable to locate repository root (looked for loadlynx.ioc)");
        }
    }
}
