use crate::config::Config;
use crate::model::{AfterPolicy, ClientRequest, ClientResponse, McuKind};
use crate::paths::Paths;
use crate::port_cache;
use crate::process::run_mcu_cmd;
use crate::timefmt::{Clock, Timestamp};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, FixedOffset};
use fs2::FileExt;
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBuf};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::time::Duration;

pub struct Server;

impl Server {
    pub async fn run() -> Result<()> {
        let paths = Paths::new()?;
        paths.ensure_dirs()?;
        let config = Config::load(paths.root())?;
        if paths.sock.exists() {
            let _ = std::fs::remove_file(&paths.sock);
        }
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(paths.lock_path())?;
        lock_file
            .try_lock_exclusive()
            .context("another instance running")?;

        let clock = Clock::new();
        let listener = UnixListener::bind(&paths.sock)?;
        println!("mcu-agentd listening at {:?}", paths.sock);
        let running = Arc::new(AtomicBool::new(true));

        // heartbeat task
        let hb_paths = paths.clone();
        let hb_clock = clock;
        let hb_cfg = config.clone();
        let hb_running = running.clone();
        tokio::spawn(async move {
            heartbeat_task(hb_paths, hb_clock, hb_cfg, hb_running)
                .await
                .ok();
        });

        while running.load(Ordering::SeqCst) {
            let (stream, _) = listener.accept().await?;
            let paths_cl = paths.clone();
            let clock_cl = clock;
            let running_cl = running.clone();
            let cfg_cl = config.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_conn(stream, paths_cl, clock_cl, running_cl, cfg_cl).await {
                    eprintln!("conn error: {e:#}");
                }
            });
        }
        Ok(())
    }

    pub async fn spawn_background() -> Result<()> {
        let paths = Paths::new()?;
        paths.ensure_dirs()?;
        let _cfg = Config::load(paths.root())?;
        // If a stale socket exists while no server listens, remove it.
        if paths.sock.exists() && UnixStream::connect(&paths.sock).await.is_err() {
            let _ = std::fs::remove_file(&paths.sock);
        }

        let exe = std::env::current_exe()?;
        let log_path = paths.log_file();
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let log_err = log_file.try_clone()?;
        let mut cmd = Command::new(exe);
        cmd.arg("serve")
            .current_dir(paths.root())
            .stdout(std::process::Stdio::from(log_file))
            .stderr(std::process::Stdio::from(log_err))
            .stdin(std::process::Stdio::null());
        cmd.spawn().context("spawn daemon")?;

        // Poll for readiness (socket connectable) up to 1s.
        let start = std::time::Instant::now();
        loop {
            if UnixStream::connect(&paths.sock).await.is_ok() {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(1) {
                anyhow::bail!("daemon failed to start (socket not ready)");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        Ok(())
    }

    pub async fn try_stop() -> Result<ClientResponse> {
        let paths = Paths::new()?;
        // fast-path: if connect succeeds, ask shutdown
        if let Ok(resp) = Server::client_send(ClientRequest::Shutdown).await {
            return Ok(resp);
        }

        // attempt to clean stale socket/lock
        if paths.sock.exists() {
            let _ = std::fs::remove_file(&paths.sock);
        }
        // try locking the lock file; if lock succeeds, release (stale)
        if let Ok(lock) = OpenOptions::new()
            .create(true)
            .write(true)
            .open(paths.lock_path())
        {
            if lock.try_lock_exclusive().is_ok() {
                // stale lock; release by dropping
                drop(lock);
            }
        }
        Ok(ClientResponse::ok(
            json!({"status": "not running", "cleaned": true}),
        ))
    }

    pub async fn client_send(req: ClientRequest) -> Result<ClientResponse> {
        let paths = Paths::new()?;
        let stream = UnixStream::connect(&paths.sock).await?;
        let mut stream = stream;
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        stream.write_all(line.as_bytes()).await?;
        let mut reader = TokioBuf::new(stream);
        let mut resp_line = String::new();
        reader.read_line(&mut resp_line).await?;
        let resp: ClientResponse = serde_json::from_str(&resp_line)?;
        Ok(resp)
    }
}

async fn handle_conn(
    stream: UnixStream,
    paths: Paths,
    clock: Clock,
    running: Arc<AtomicBool>,
    config: Config,
) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = TokioBuf::new(read_half);
    let mut buf = String::new();
    reader.read_line(&mut buf).await?;
    let req: ClientRequest = serde_json::from_str(&buf)?;
    let resp = handle_request(req, &paths, &clock, &running, &config)
        .await
        .unwrap_or_else(|e| ClientResponse::err(format!("{e:#}")));
    let line = serde_json::to_string(&resp)? + "\n";
    write_half.write_all(line.as_bytes()).await?;
    Ok(())
}

async fn handle_request(
    req: ClientRequest,
    paths: &Paths,
    clock: &Clock,
    running: &Arc<AtomicBool>,
    config: &Config,
) -> Result<ClientResponse> {
    match req {
        ClientRequest::Shutdown => {
            write_shutdown_meta(paths, clock)?;
            std::fs::remove_file(&paths.sock).ok();
            running.store(false, Ordering::SeqCst);
            Ok(ClientResponse::ok(json!({"status":"stopping"})))
        }
        ClientRequest::Status => {
            let ts = clock.now();
            Ok(ClientResponse::ok(json!({
                "ts": ts.iso(),
                "pid": std::process::id(),
                "sock": paths.sock,
            })))
        }
        ClientRequest::SetPort { mcu, path } => {
            port_cache::write_port(paths, mcu.clone(), path.to_string_lossy().as_ref())?;
            let ts = clock.now();
            Ok(ClientResponse::ok(
                json!({"ts": ts.iso(), "mcu": mcu, "path": path}),
            ))
        }
        ClientRequest::GetPort { mcu } => {
            let ts = clock.now();
            let val = port_cache::read_port(paths, mcu.clone())?;
            Ok(ClientResponse::ok(
                json!({"ts": ts.iso(), "mcu": mcu, "path": val}),
            ))
        }
        ClientRequest::ListPorts { mcu } => {
            let list = list_ports(paths, &mcu).await?;
            Ok(ClientResponse::ok(json!({"mcu": mcu, "ports": list})))
        }
        ClientRequest::Flash { mcu, elf, after } => {
            let ts = clock.now();
            let res =
                flash_mcu(paths, &mcu, elf, after.unwrap_or(AfterPolicy::NoReset), &ts).await?;
            Ok(ClientResponse::ok(res))
        }
        ClientRequest::Reset { mcu } => {
            let ts = clock.now();
            let res = reset_mcu(paths, &mcu, &ts).await?;
            Ok(ClientResponse::ok(res))
        }
        ClientRequest::Monitor {
            mcu,
            elf,
            duration,
            lines,
        } => {
            let ts = clock.now();
            let res = monitor_mcu(paths, &mcu, elf, duration, lines, &ts).await?;
            Ok(ClientResponse::ok(res))
        }
        ClientRequest::Logs {
            mcu,
            since,
            until,
            tail,
            sessions,
        } => {
            let effective_tail = tail.unwrap_or(config.tail_default);
            let entries = query_logs(
                paths,
                mcu,
                since.as_deref(),
                until.as_deref(),
                Some(effective_tail),
            )?;
            let sessions_payload = if sessions {
                query_session_logs(paths, &entries, effective_tail)?
            } else {
                json!([])
            };
            Ok(ClientResponse::ok(
                json!({"meta": entries, "sessions": sessions_payload}),
            ))
        }
    }
}

async fn list_ports(_paths: &Paths, mcu: &McuKind) -> Result<Vec<String>> {
    match mcu {
        McuKind::Digital => {
            // Use espflash list-ports
            let output = Command::new("espflash")
                .arg("list-ports")
                .output()
                .await
                .context("espflash list-ports")?;
            if !output.status.success() {
                return Err(anyhow!("espflash list-ports failed: {}", output.status));
            }
            let text = String::from_utf8_lossy(&output.stdout);
            let ports: Vec<String> = text
                .lines()
                .filter_map(|l| l.trim().split_whitespace().next())
                .filter(|s| s.starts_with('/'))
                .map(|s| s.to_string())
                .collect();
            Ok(ports)
        }
        McuKind::Analog => {
            let output = Command::new("probe-rs")
                .arg("list")
                .output()
                .await
                .context("probe-rs list")?;
            if !output.status.success() {
                return Err(anyhow!("probe-rs list failed: {}", output.status));
            }
            let text = String::from_utf8_lossy(&output.stdout);
            let sels: Vec<String> = text
                .lines()
                .filter_map(|l| l.split("-- ").nth(1))
                .filter_map(|rest| rest.split_whitespace().next())
                .map(|s| s.to_string())
                .collect();
            Ok(sels)
        }
    }
}

fn write_meta(
    paths: &Paths,
    mcu: &McuKind,
    ts: &Timestamp,
    event: &str,
    res: &crate::process::RunResult,
) -> Result<()> {
    let meta_path = paths.meta(mcu.clone());
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(meta_path)?;
    let line = json!({
        "ts": ts.iso(),
        "mono_ms": ts.mono_ms(),
        "mcu": match mcu { McuKind::Digital => "digital", McuKind::Analog => "analog" },
        "event": event,
        "status": res.status,
        "duration_ms": res.duration_ms,
        "session": res.session_file,
    });
    writeln!(f, "{}", serde_json::to_string(&line)?)?;
    Ok(())
}

async fn ensure_elf(paths: &Paths, mcu: &McuKind, elf: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = elf {
        if p.exists() {
            return Ok(p);
        }
        return Err(anyhow!("ELF not found: {:?}", p));
    }
    match mcu {
        McuKind::Digital => {
            let p = paths
                .root()
                .join("firmware/digital/target/xtensa-esp32s3-none-elf/release/digital");
            if p.exists() {
                return Ok(p);
            }
            let status = Command::new("make")
                .arg("d-build")
                .current_dir(paths.root())
                .status()
                .await?;
            if !status.success() {
                return Err(anyhow!("make d-build failed: {status}"));
            }
            Ok(p)
        }
        McuKind::Analog => {
            let p = paths
                .root()
                .join("firmware/analog/target/thumbv7em-none-eabihf/release/analog");
            if p.exists() {
                return Ok(p);
            }
            let status = Command::new("make")
                .arg("a-build")
                .current_dir(paths.root())
                .status()
                .await?;
            if !status.success() {
                return Err(anyhow!("make a-build failed: {status}"));
            }
            Ok(p)
        }
    }
}

async fn flash_mcu(
    paths: &Paths,
    mcu: &McuKind,
    elf: Option<PathBuf>,
    after: AfterPolicy,
    ts: &Timestamp,
) -> Result<serde_json::Value> {
    // Ensure ELF exists/builds; we need the path for flash.
    let elf_path = ensure_elf(paths, mcu, elf).await?;

    // Analog flash endurance gate: skip if version unchanged.
    if *mcu == McuKind::Analog {
        let build_ver = read_trim(&paths.analog_fw_version);
        let last_ver = read_trim(&paths.analog_last_flashed);
        if let (Some(b), Some(l)) = (build_ver.clone(), last_ver.clone()) {
            if b == l {
                let dummy = crate::process::RunResult {
                    status: 0,
                    duration_ms: 0,
                    session_file: PathBuf::new(),
                };
                write_meta(paths, mcu, ts, "flash-skip", &dummy)?;
                return Ok(json!({
                    "ts": ts.iso(),
                    "mcu": mcu,
                    "status": 0,
                    "skipped": true,
                    "reason": "analog version unchanged",
                    "version": b,
                }));
            }
        }
    }
    let cmd = match mcu {
        McuKind::Digital => {
            let port = require_port(paths, McuKind::Digital)?;
            let mut c = Command::new("espflash");
            c.arg("flash")
                .arg(&elf_path)
                .arg("--chip")
                .arg("esp32s3")
                .arg("--port")
                .arg(port)
                .arg("--after")
                .arg(match after {
                    AfterPolicy::NoReset => "no-reset",
                    AfterPolicy::HardReset => "hard-reset",
                })
                .arg("--ignore_app_descriptor")
                .arg("--non-interactive")
                .arg("--skip-update-check");
            c
        }
        McuKind::Analog => {
            let probe = require_port(paths, McuKind::Analog)?;
            let mut c = Command::new("probe-rs");
            c.arg("download")
                .arg("--chip")
                .arg("STM32G431CB")
                .arg("--probe")
                .arg(probe)
                .arg(&elf_path);
            c
        }
    };
    let res = run_mcu_cmd(paths, mcu, ts, cmd, None, None).await?;
    // flash succeeded; update last-flashed markers if version available
    if *mcu == McuKind::Analog {
        if let Some(ver) = read_trim(&paths.analog_fw_version) {
            let _ = std::fs::write(&paths.analog_last_flashed, ver);
        }
    } else {
        if let Some(ver) = read_trim(&paths.digital_fw_version) {
            let _ = std::fs::write(&paths.digital_last_flashed, ver);
        }
    }
    write_meta(paths, mcu, ts, "flash", &res)?;
    Ok(json!({
        "ts": ts.iso(),
        "mcu": mcu,
        "status": res.status,
        "duration_ms": res.duration_ms,
        "session": res.session_file,
    }))
}

async fn reset_mcu(paths: &Paths, mcu: &McuKind, ts: &Timestamp) -> Result<serde_json::Value> {
    let cmd = match mcu {
        McuKind::Digital => {
            let port = require_port(paths, McuKind::Digital)?;
            let mut c = Command::new("espflash");
            c.arg("reset")
                .arg("--chip")
                .arg("esp32s3")
                .arg("--port")
                .arg(port);
            c
        }
        McuKind::Analog => {
            let probe = require_port(paths, McuKind::Analog)?;
            let mut c = Command::new("probe-rs");
            c.arg("reset")
                .arg("--chip")
                .arg("STM32G431CB")
                .arg("--probe")
                .arg(probe);
            c
        }
    };
    let res = run_mcu_cmd(paths, mcu, ts, cmd, None, None).await?;
    write_meta(paths, mcu, ts, "reset", &res)?;
    Ok(json!({
        "ts": ts.iso(),
        "mcu": mcu,
        "status": res.status,
        "duration_ms": res.duration_ms,
        "session": res.session_file,
    }))
}

async fn monitor_mcu(
    paths: &Paths,
    mcu: &McuKind,
    elf: Option<PathBuf>,
    duration_ms: Option<u64>,
    lines: Option<usize>,
    ts: &Timestamp,
) -> Result<serde_json::Value> {
    let _elf_path = ensure_elf(paths, mcu, elf).await?;
    let cmd = match mcu {
        McuKind::Digital => {
            let port = require_port(paths, McuKind::Digital)?;
            let cfg = Config::load(paths.root())?;
            let mut c = Command::new("make");
            c.arg("d-run")
                .current_dir(paths.root())
                .env("PORT", port)
                .env("ESPFLASH_ARGS", cfg.espflash_args);
            c
        }
        McuKind::Analog => {
            let probe = require_port(paths, McuKind::Analog)?;
            let mut c = Command::new("make");
            c.arg("a-reset-attach")
                .current_dir(paths.root())
                .env("PROBE", probe);
            c
        }
    };

    let duration = duration_ms.map(Duration::from_millis);
    let res = run_mcu_cmd(paths, mcu, ts, cmd, duration, lines).await?;
    write_meta(paths, mcu, ts, "monitor", &res)?;
    Ok(json!({
        "ts": ts.iso(),
        "mcu": mcu,
        "status": res.status,
        "duration_ms": res.duration_ms,
        "session": res.session_file,
    }))
}

fn require_port(paths: &Paths, mcu: McuKind) -> Result<String> {
    if let Some(val) = port_cache::read_port(paths, mcu.clone())? {
        return Ok(val);
    }
    // try helper scripts for convenience
    let script = match mcu {
        McuKind::Digital => paths.root().join("scripts/ensure_esp32_port.sh"),
        McuKind::Analog => paths.root().join("scripts/ensure_stm32_probe.sh"),
    };
    if script.exists() {
        let output = std::process::Command::new(&script)
            .current_dir(paths.root())
            .output()
            .context("run ensure script")?;
        if output.status.success() {
            let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !val.is_empty() {
                return Ok(val);
            }
        }
    }
    Err(anyhow!("port/probe not set; please run set-port"))
}

// Minimal log querying: tail from meta files if needed later; stub for now.
fn _query_logs(_paths: &Paths) -> Result<Vec<serde_json::Value>> {
    Ok(vec![])
}

fn read_trim(path: &PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_rfc3339(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s).ok()
}

fn within(
    ts: &str,
    since: Option<DateTime<FixedOffset>>,
    until: Option<DateTime<FixedOffset>>,
) -> bool {
    let parsed = match parse_rfc3339(ts) {
        Some(v) => v,
        None => return false,
    };
    if let Some(s) = since {
        if parsed < s {
            return false;
        }
    }
    if let Some(u) = until {
        if parsed > u {
            return false;
        }
    }
    true
}

fn query_logs(
    paths: &Paths,
    mcu: Option<McuKind>,
    since: Option<&str>,
    until: Option<&str>,
    tail: Option<usize>,
) -> Result<serde_json::Value> {
    let since_dt = since.and_then(parse_rfc3339);
    let until_dt = until.and_then(parse_rfc3339);
    let mut entries = Vec::new();

    let metas: Vec<(McuKind, &std::path::Path)> = match mcu {
        Some(McuKind::Digital) => vec![(McuKind::Digital, paths.meta(McuKind::Digital))],
        Some(McuKind::Analog) => vec![(McuKind::Analog, paths.meta(McuKind::Analog))],
        None => vec![
            (McuKind::Digital, paths.meta(McuKind::Digital)),
            (McuKind::Analog, paths.meta(McuKind::Analog)),
        ],
    };

    for (kind, path) in metas {
        if !path.exists() {
            continue;
        }
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines().filter_map(Result::ok) {
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&line) {
                let ts_str = v
                    .get("ts")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                if !within(&ts_str, since_dt, until_dt) {
                    continue;
                }
                // inject mcu if missing
                if v.get("mcu").is_none() {
                    v["mcu"] = serde_json::Value::String(
                        match kind {
                            McuKind::Digital => "digital",
                            McuKind::Analog => "analog",
                        }
                        .to_string(),
                    );
                }
                let sort_ts = parse_rfc3339(&ts_str);
                entries.push((sort_ts, v));
            }
        }
    }

    // Sort by timestamp ascending when available
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut vals: Vec<serde_json::Value> = entries.into_iter().map(|(_, v)| v).collect();
    if let Some(t) = tail {
        if vals.len() > t {
            vals = vals.split_off(vals.len() - t);
        }
    }

    Ok(serde_json::Value::Array(vals))
}

fn query_session_logs(
    _paths: &Paths,
    meta_entries: &serde_json::Value,
    tail: usize,
) -> Result<serde_json::Value> {
    let mut session_lines = Vec::new();
    let arr = meta_entries.as_array().cloned().unwrap_or_default();
    for entry in arr {
        if let Some(sess_path) = entry.get("session").and_then(|s| s.as_str()) {
            let p = PathBuf::from(sess_path);
            if !p.exists() {
                continue;
            }
            let file = File::open(&p)?;
            let reader = BufReader::new(file);
            let mut buf: Vec<String> = reader.lines().filter_map(Result::ok).collect();
            if buf.len() > tail {
                buf = buf.split_off(buf.len() - tail);
            }
            for l in buf {
                session_lines.push(l);
            }
        }
    }
    Ok(serde_json::Value::Array(
        session_lines
            .into_iter()
            .map(serde_json::Value::from)
            .collect(),
    ))
}

async fn heartbeat_task(
    paths: Paths,
    clock: Clock,
    config: Config,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let interval = Duration::from_secs(config.heartbeat_secs);
    while running.load(Ordering::SeqCst) {
        let ts = clock.now();
        let dummy = crate::process::RunResult {
            status: 0,
            duration_ms: 0,
            session_file: PathBuf::new(),
        };
        let _ = write_meta(&paths, &McuKind::Digital, &ts, "heartbeat", &dummy);
        let _ = write_meta(&paths, &McuKind::Analog, &ts, "heartbeat", &dummy);
        tokio::time::sleep(interval).await;
    }
    Ok(())
}

fn write_shutdown_meta(paths: &Paths, clock: &Clock) -> Result<()> {
    let ts = clock.now();
    let dummy = crate::process::RunResult {
        status: 0,
        duration_ms: 0,
        session_file: PathBuf::new(),
    };
    let _ = write_meta(paths, &McuKind::Digital, &ts, "shutdown", &dummy);
    let _ = write_meta(paths, &McuKind::Analog, &ts, "shutdown", &dummy);
    Ok(())
}
