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
use std::collections::HashMap;
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
use tokio::sync::{Mutex, watch};
use tokio::time::Duration;

pub struct Server;

#[derive(Clone)]
struct DaemonState {
    monitors: Arc<Mutex<HashMap<McuKind, MonitorTask>>>,
    config: Config,
}

#[derive(Clone)]
struct MonitorTask {
    cancel: watch::Sender<bool>,
    log_path: PathBuf,
}

impl Server {
    pub async fn run() -> Result<()> {
        let paths = Paths::new()?;
        paths.ensure_dirs()?;
        let config = Config::default();
        let state = DaemonState {
            monitors: Arc::new(Mutex::new(HashMap::new())),
            config,
        };
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
        println!("loadlynx-agentd listening at {:?}", paths.sock);
        let running = Arc::new(AtomicBool::new(true));
        // start background monitors if ports cached
        start_cached_monitors(&paths, &state, &clock).await.ok();

        while running.load(Ordering::SeqCst) {
            let (stream, _) = listener.accept().await?;
            let paths_cl = paths.clone();
            let clock_cl = clock;
            let running_cl = running.clone();
            let state_cl = state.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_conn(stream, paths_cl, clock_cl, running_cl, state_cl).await
                {
                    eprintln!("conn error: {e:#}");
                }
            });
        }
        Ok(())
    }

    pub async fn spawn_background() -> Result<()> {
        let paths = Paths::new()?;
        paths.ensure_dirs()?;
        let _cfg = Config::default();
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
    state: DaemonState,
) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = TokioBuf::new(read_half);
    let mut buf = String::new();
    reader.read_line(&mut buf).await?;
    let req: ClientRequest = serde_json::from_str(&buf)?;
    let resp = handle_request(req, &paths, &clock, &running, &state)
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
    state: &DaemonState,
) -> Result<ClientResponse> {
    match req {
        ClientRequest::Shutdown => {
            std::fs::remove_file(&paths.sock).ok();
            running.store(false, Ordering::SeqCst);
            stop_all_monitors(paths, state).await.ok();
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
            // Hot-restart monitor so new port takes effect immediately.
            stop_monitor(paths, state, &mcu).await.ok();
            if let Err(e) = start_monitor_if_cached(paths, state, &mcu, &ts).await {
                eprintln!("monitor restart after set-port failed: {e:#}");
            }
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
            let res = flash_mcu(
                paths,
                state,
                &mcu,
                elf,
                after.unwrap_or(AfterPolicy::NoReset),
                &ts,
            )
            .await?;
            Ok(ClientResponse::ok(res))
        }
        ClientRequest::Reset { mcu } => {
            let ts = clock.now();
            let res = reset_mcu(paths, state, &mcu, &ts).await?;
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
            let effective_tail = tail.unwrap_or(state.config.tail_default);
            let entries = query_logs(
                paths,
                mcu,
                since.as_deref(),
                until.as_deref(),
                Some(effective_tail),
            )?;
            let sessions_payload = if sessions {
                query_session_logs(
                    paths,
                    &entries,
                    since.as_deref(),
                    until.as_deref(),
                    Some(effective_tail),
                )?
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
            Err(anyhow!("default ELF missing; provide --elf"))
        }
        McuKind::Analog => {
            let p = paths
                .root()
                .join("firmware/analog/target/thumbv7em-none-eabihf/release/analog");
            if p.exists() {
                return Ok(p);
            }
            Err(anyhow!("default ELF missing; provide --elf"))
        }
    }
}

async fn flash_mcu(
    paths: &Paths,
    state: &DaemonState,
    mcu: &McuKind,
    elf: Option<PathBuf>,
    after: AfterPolicy,
    ts: &Timestamp,
) -> Result<serde_json::Value> {
    stop_monitor(paths, state, mcu).await.ok();
    let elf_path = ensure_elf(paths, mcu, elf).await?;
    let res = match mcu {
        McuKind::Digital => {
            let port = require_port(paths, McuKind::Digital)?;
            let mut cmd = Command::new("espflash");
            cmd.arg("flash")
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
            run_mcu_cmd(paths, mcu, ts, cmd, None, None, None, None).await?
        }
        McuKind::Analog => {
            let probe = require_port(paths, McuKind::Analog)?;
            // Retry STM32G4 flash when probe-rs reports the USB interface is busy.
            let mut attempt = 0usize;
            loop {
                let mut cmd = Command::new("probe-rs");
                cmd.arg("download")
                    .arg("--chip")
                    .arg("STM32G431CB")
                    .arg("--probe")
                    .arg(probe.clone())
                    .arg(&elf_path);
                let res_try = run_mcu_cmd(paths, mcu, ts, cmd, None, None, None, None).await?;
                if res_try.status == 0
                    || attempt >= 2
                    || !session_has_usb_claim_error(&res_try.session_file)
                {
                    break res_try;
                }
                attempt += 1;
                // Give the OS/probe a bit of time to release the USB interfaces.
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    };
    write_meta(paths, mcu, ts, "flash", &res)?;
    if res.status != 0 {
        return Err(anyhow!(
            "flash command exited with status {} (see {})",
            res.status,
            res.session_file.display()
        ));
    }
    start_monitor_if_cached(paths, state, mcu, ts).await.ok();
    Ok(json!({
        "ts": ts.iso(),
        "mcu": mcu,
        "status": res.status,
        "duration_ms": res.duration_ms,
        "session": res.session_file,
    }))
}

async fn reset_mcu(
    paths: &Paths,
    state: &DaemonState,
    mcu: &McuKind,
    ts: &Timestamp,
) -> Result<serde_json::Value> {
    stop_monitor(paths, state, mcu).await.ok();
    let res = match mcu {
        McuKind::Digital => {
            let port = require_port(paths, McuKind::Digital)?;
            let mut cmd = Command::new("espflash");
            cmd.arg("reset")
                .arg("--chip")
                .arg("esp32s3")
                .arg("--port")
                .arg(port);
            run_mcu_cmd(paths, mcu, ts, cmd, None, None, None, None).await?
        }
        McuKind::Analog => {
            let probe = require_port(paths, McuKind::Analog)?;
            // Retry STM32G4 reset on transient USB/probe busy errors and a known
            // CMSIS-DAP "Info" glitch where the command ID in the response does
            // not match the sent command ID. In both cases a short delay and
            // retry usually succeed.
            let mut attempt = 0usize;
            loop {
                let mut cmd = Command::new("probe-rs");
                cmd.arg("reset")
                    .arg("--chip")
                    .arg("STM32G431CB")
                    .arg("--probe")
                    .arg(probe.clone());
                let res_try = run_mcu_cmd(paths, mcu, ts, cmd, None, None, None, None).await?;

                let has_usb_claim = session_has_usb_claim_error(&res_try.session_file);
                let has_cmsisdap_info = session_has_cmsisdap_info_error(&res_try.session_file);

                // Success: return immediately.
                if res_try.status == 0 {
                    break res_try;
                }

                // For known transient probe issues (USB interface claimed or CMSIS-DAP
                // Info mismatch), retry a couple of times before giving up.
                if (has_usb_claim || has_cmsisdap_info) && attempt < 2 {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }

                // Non-retryable error (or retries exhausted): surface the last result.
                break res_try;
            }
        }
    };

    // For Analog, if we still see probe/USB issues after retries, emit a warning but
    // keep the non-zero status so the CLI sees a hard failure. The helpers only try to
    // classify known transient errors; they must not turn them into soft successes.
    if matches!(mcu, McuKind::Analog) && res.status != 0 {
        if session_has_usb_claim_error(&res.session_file) {
            eprintln!(
                "warn: analog reset hit USB interface claimed, retries exhausted; reporting failure"
            );
        } else if session_has_cmsisdap_info_error(&res.session_file) {
            eprintln!(
                "warn: analog reset hit CMSIS-DAP Info error, retries exhausted; reporting failure"
            );
        }
    }
    write_meta(paths, mcu, ts, "reset", &res)?;
    if res.status != 0 {
        return Err(anyhow!(
            "reset command exited with status {} (see {})",
            res.status,
            res.session_file.display()
        ));
    }
    start_monitor_if_cached(paths, state, mcu, ts).await.ok();
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
    let latest = latest_log(paths, mcu, elf.as_ref())?;
    let duration = duration_ms.map(Duration::from_millis);
    let lines = lines.unwrap_or(0);
    Ok(json!({
        "ts": ts.iso(),
        "mcu": mcu,
        "path": latest,
        "duration_ms": duration.map(|d| d.as_millis()),
        "lines": if lines==0 { None::<usize> } else { Some(lines) },
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
    since: Option<&str>,
    until: Option<&str>,
    tail: Option<usize>,
) -> Result<serde_json::Value> {
    let mut results = Vec::new();
    let arr = meta_entries.as_array().cloned().unwrap_or_default();
    for entry in arr {
        if let Some(sess_path) = entry.get("session").and_then(|s| s.as_str()) {
            let p = PathBuf::from(sess_path);
            if !p.exists() {
                continue;
            }
            let lines = read_session(&p, since, until, tail)?;
            results.push(json!({"session": p, "lines": lines}));
        }
    }
    Ok(serde_json::Value::Array(results))
}

async fn start_cached_monitors(paths: &Paths, state: &DaemonState, clock: &Clock) -> Result<()> {
    for mcu in [McuKind::Digital, McuKind::Analog] {
        start_monitor_if_cached(paths, state, &mcu, &clock.now())
            .await
            .ok();
    }
    Ok(())
}

async fn start_monitor_if_cached(
    paths: &Paths,
    state: &DaemonState,
    mcu: &McuKind,
    ts: &Timestamp,
) -> Result<()> {
    let port = match port_cache::read_port(paths, mcu.clone())? {
        Some(p) => p,
        None => return Ok(()),
    };
    let mut map = state.monitors.lock().await;
    if map.contains_key(mcu) {
        return Ok(());
    }
    let elf = ensure_elf(paths, mcu, None).await?;
    let log_path = monitor_file_path(paths, mcu, ts);
    let (tx, rx) = watch::channel(false);
    let paths_cl = paths.clone();
    let mcu_cl = mcu.clone();
    let ts_cl = ts.clone();
    let elf_cl = elf.clone();
    let port_cl = port.clone();
    let log_path_spawn = log_path.clone();
    tokio::spawn(async move {
        let cmd = match mcu_cl {
            McuKind::Digital => {
                let mut c = Command::new("espflash");
                c.arg("monitor")
                    .arg("--chip")
                    .arg("esp32s3")
                    .arg("--port")
                    .arg(port_cl)
                    .arg("--elf")
                    .arg(elf_cl)
                    .arg("--log-format")
                    .arg("defmt")
                    .arg("--non-interactive")
                    .arg("--skip-update-check")
                    .arg("--after")
                    .arg("no-reset");
                c
            }
            McuKind::Analog => {
                let mut c = Command::new("probe-rs");
                c.arg("run")
                    .arg("--chip")
                    .arg("STM32G431CB")
                    .arg("--probe")
                    .arg(port_cl)
                    .arg("--log-format")
                    .arg("oneline")
                    .arg(elf_cl);
                c
            }
        };
        let _ = run_mcu_cmd(
            &paths_cl,
            &mcu_cl,
            &ts_cl,
            cmd,
            None,
            None,
            Some(rx),
            Some(log_path_spawn),
        )
        .await;
    });
    write_meta(
        paths,
        mcu,
        ts,
        "monitor-start",
        &crate::process::RunResult {
            status: 0,
            duration_ms: 0,
            session_file: log_path.clone(),
        },
    )?;
    map.insert(
        mcu.clone(),
        MonitorTask {
            cancel: tx,
            log_path,
        },
    );
    Ok(())
}

async fn stop_monitor(paths: &Paths, state: &DaemonState, mcu: &McuKind) -> Result<()> {
    let mut map = state.monitors.lock().await;
    if let Some(task) = map.remove(mcu) {
        let _ = task.cancel.send(true);
        write_meta(
            paths,
            mcu,
            &Clock::new().now(),
            "monitor-stop",
            &crate::process::RunResult {
                status: 0,
                duration_ms: 0,
                session_file: task.log_path,
            },
        )?;
    }
    Ok(())
}

async fn stop_all_monitors(paths: &Paths, state: &DaemonState) -> Result<()> {
    for mcu in [McuKind::Digital, McuKind::Analog] {
        stop_monitor(paths, state, &mcu).await.ok();
    }
    Ok(())
}

fn monitor_file_path(paths: &Paths, mcu: &McuKind, ts: &Timestamp) -> PathBuf {
    let dir = paths.monitor_dir(mcu.clone());
    let filename = format!("{}.mon.log", ts.wall.format("%Y%m%d_%H%M%S"));
    dir.join(filename)
}

fn latest_log(paths: &Paths, mcu: &McuKind, _elf: Option<&PathBuf>) -> Result<PathBuf> {
    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    let dirs = vec![
        paths.monitor_dir(mcu.clone()).to_path_buf(),
        paths.session_dir(mcu.clone()).to_path_buf(),
    ];
    for d in dirs {
        if !d.exists() {
            continue;
        }
        for entry in std::fs::read_dir(d)? {
            let e = entry?;
            let md = e.metadata()?;
            if md.is_file() {
                if let Ok(mt) = md.modified() {
                    if latest.as_ref().map(|(t, _)| mt > *t).unwrap_or(true) {
                        latest = Some((mt, e.path()));
                    }
                }
            }
        }
    }
    latest
        .map(|(_, p)| p)
        .ok_or_else(|| anyhow!("no logs found for {:?}", mcu))
}

fn session_has_usb_claim_error(path: &PathBuf) -> bool {
    if let Ok(text) = std::fs::read_to_string(path) {
        let needles = [
            "could not be opened for exclusive access",
            "interfaces are claimed",
        ];
        needles.iter().any(|n| text.contains(n))
    } else {
        false
    }
}

fn session_has_cmsisdap_info_error(path: &PathBuf) -> bool {
    if let Ok(text) = std::fs::read_to_string(path) {
        // Typical probe-rs CMSIS-DAP Info glitch sequence:
        //   Error: Failed to open probe: Failed to open the debug probe.
        //   Caused by:
        //     0: An error which is specific to the debug probe in use occurred.
        //     1: Error handling CMSIS-DAP command Info.
        //     2: Command ID in response (0x2) does not match sent command ID (Info - 0x0).
        text.contains("Error handling CMSIS-DAP command Info")
            && text.contains(
                "Command ID in response (0x2) does not match sent command ID (Info - 0x0)",
            )
    } else {
        false
    }
}

fn read_session(
    path: &PathBuf,
    since: Option<&str>,
    until: Option<&str>,
    tail: Option<usize>,
) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| session_ts_ok_json(l, since, until))
        .collect();
    if let Some(n) = tail {
        if lines.len() > n {
            lines = lines.split_off(lines.len() - n);
        }
    }
    Ok(lines)
}

fn session_ts_ok_json(line: &str, since: Option<&str>, until: Option<&str>) -> bool {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(ts) = v.get("ts").and_then(|t| t.as_str()) {
            if let Some(s) = since {
                if ts < s {
                    return false;
                }
            }
            if let Some(u) = until {
                if ts > u {
                    return false;
                }
            }
        }
    }
    true
}
