use crate::model::McuKind;
use crate::paths::Paths;
use crate::timefmt::Timestamp;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
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
) -> Result<RunResult> {
    let session_file = session_file_path(paths, mcu, ts);
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
            line = out_reader.next_line() => {
                match line? {
                    Some(l) => {
                        lines_seen += 1;
                        let pref = prefix(&ts.iso(), mcu, "log");
                        writeln!(file, "{} {}", pref, l)?;
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
                        let pref = prefix(&ts.iso(), mcu, "err");
                        writeln!(file, "{} {}", pref, l)?;
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

fn prefix(ts: &str, mcu: &McuKind, event: &str) -> String {
    format!(
        "{{\"ts\":\"{}\",\"mcu\":\"{}\",\"event\":\"{}\"}}",
        ts,
        match mcu {
            McuKind::Digital => "digital",
            McuKind::Analog => "analog",
        },
        event
    )
}
