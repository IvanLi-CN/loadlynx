use crate::model::McuKind;
use crate::paths::Paths;
use crate::timefmt::Timestamp;
use anyhow::{Context, Result};
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::time::{Instant, sleep_until};

#[derive(Debug)]
pub struct RunResult {
    pub status: i32,
    pub duration_ms: u128,
    pub session_file: PathBuf,
}

pub async fn run_mcu_cmd(
    paths: &Paths,
    mcu: &McuKind,
    ts: &Timestamp,
    mut cmd: Command,
    duration: Option<std::time::Duration>,
    line_limit: Option<usize>,
    cancel: Option<watch::Receiver<bool>>,
    log_path_override: Option<PathBuf>,
) -> Result<RunResult> {
    let session_file = log_path_override.unwrap_or_else(|| session_file_path(paths, mcu, ts));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session_file)
        .with_context(|| format!("open session log {:?}", session_file))?;

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().context("spawn command")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut out_reader = BufReader::new(stdout).lines();
    let mut err_reader = BufReader::new(stderr).lines();

    let start = Instant::now();
    let deadline = duration.map(|d| Instant::now() + d);
    let mut lines_seen: usize = 0;
    let mut timed_out = false;

    let mut cancel_rx = cancel;
    loop {
        tokio::select! {
            _ = async {
                if let Some(dl) = deadline {
                    sleep_until(dl).await;
                }
            }, if deadline.is_some() => {
                timed_out = true;
                let _ = child.start_kill();
                break;
            }
            _ = async {
                if let Some(rx) = cancel_rx.as_mut() {
                    let _ = rx.changed().await;
                }
            }, if cancel_rx.is_some() => {
                let _ = child.start_kill();
                break;
            }
            line = out_reader.next_line() => {
                match line? {
                    Some(l) => {
                        lines_seen += 1;
                        let line_json = log_json(&ts.iso(), mcu, "stdout", &l);
                        writeln!(file, "{}", line_json)?;
                        if line_limit.map(|lim| lines_seen >= lim).unwrap_or(false) {
                            let _ = child.start_kill();
                            break;
                        }
                    }
                    None => break,
                }
            }
            line = err_reader.next_line() => {
                match line? {
                    Some(l) => {
                        let line_json = log_json(&ts.iso(), mcu, "stderr", &l);
                        writeln!(file, "{}", line_json)?;
                    }
                    None => break,
                }
            }
        }
    }

    let status = match child.wait().await {
        Ok(s) => s.code().unwrap_or(if timed_out { -2 } else { -1 }),
        Err(_) => -1,
    };
    let dur = start.elapsed();

    Ok(RunResult {
        status,
        duration_ms: dur.as_millis(),
        session_file,
    })
}

fn session_file_path(paths: &Paths, mcu: &McuKind, ts: &Timestamp) -> PathBuf {
    let dir = paths.session_dir(mcu.clone());
    let filename = format!("{}.session.log", ts.wall.format("%Y%m%d_%H%M%S"));
    dir.join(filename)
}

fn log_json(ts: &str, mcu: &McuKind, src: &str, text: &str) -> String {
    format!(
        "{{\"ts\":\"{}\",\"mcu\":\"{}\",\"src\":\"{}\",\"text\":{}}}",
        ts,
        match mcu {
            McuKind::Digital => "digital",
            McuKind::Analog => "analog",
        },
        src,
        serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
    )
}
