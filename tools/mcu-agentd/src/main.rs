mod config;
mod model;
mod paths;
mod port_cache;
mod process;
mod server;
mod timefmt;

use anyhow::{Result, bail, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::{Select, theme::ColorfulTheme};
use model::{ClientRequest, McuKind};
use serde_json;
use serialport::{SerialPortType, available_ports};
use server::Server;
use std::path::PathBuf;
use std::process::exit;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::time::{Duration as TokioDuration, Instant, sleep};

/// LoadLynx agentd – single-instance helper for LoadLynx boards (ESP32-S3 + STM32G431).
#[derive(Parser, Debug)]
#[command(name = "loadlynx-agentd", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Start daemon in background (no-op if already running).
    Start,
    /// Stop daemon if running.
    Stop,
    /// Query daemon status.
    Status,
    /// Set cached port/probe selector for MCU.
    SetPort {
        #[arg(value_enum)]
        mcu: McuOpt,
        /// Leave empty to interactively select when supported.
        path: Option<PathBuf>,
    },
    /// Get cached port/probe selector for MCU.
    GetPort {
        #[arg(value_enum)]
        mcu: McuOpt,
    },
    /// List detected ports/probes for the MCU.
    ListPorts {
        #[arg(value_enum)]
        mcu: McuOpt,
    },
    /// Flash firmware to MCU (does not auto-run).
    Flash {
        #[arg(value_enum)]
        mcu: McuOpt,
        /// ELF path; if omitted, use the default release ELF (fails if it has not been built yet).
        elf: Option<PathBuf>,
        /// ESP32 after-reset policy (analog ignores).
        #[arg(long, default_value = "no-reset", value_enum)]
        after: OptionAfter,
    },
    /// Reset MCU (no flash).
    Reset {
        #[arg(value_enum)]
        mcu: McuOpt,
    },
    /// Monitor/attach MCU logs with optional timeout.
    Monitor {
        #[arg(value_enum)]
        mcu: McuOpt,
        /// Optional ELF path (reserved for future use; normally omit and rely on the default ELF).
        elf: Option<PathBuf>,
        /// Reset MCU before monitoring to capture a fresh boot log.
        #[arg(long)]
        reset: bool,
        /// Auto-stop after duration, e.g. 30s/2m/1h (0 = unlimited).
        #[arg(long, value_parser = humantime::parse_duration, default_value = "0")]
        duration: std::time::Duration,
        /// Auto-stop after N lines (0 = unlimited).
        #[arg(long, default_value = "0")]
        lines: usize,
    },
    /// Fetch meta/session logs (server-side filtered).
    Logs {
        #[arg(value_enum)]
        mcu: LogsMcuOpt,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        tail: Option<usize>,
        #[arg(
            long,
            default_value_t = false,
            help = "include session log lines (tail per session)"
        )]
        sessions: bool,
    },
    /// Internal: run daemon in foreground (do not call directly).
    #[command(hide = true)]
    Serve,
}

#[derive(Clone, Debug, ValueEnum)]
enum McuOpt {
    Digital,
    Analog,
}

#[derive(Clone, Debug, ValueEnum)]
enum LogsMcuOpt {
    Digital,
    Analog,
    All,
}

#[derive(Clone, Debug, ValueEnum)]
enum OptionAfter {
    #[value(name = "no-reset")]
    NoReset,
    #[value(name = "hard-reset")]
    HardReset,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    fn print_and_exit(resp: &model::ClientResponse) -> Result<()> {
        let out = serde_json::to_string_pretty(resp)?;
        if resp.ok {
            println!("{}", out);
            Ok(())
        } else {
            eprintln!("{}", out);
            exit(1);
        }
    }

    match cli.cmd {
        Cmd::Serve => {
            Server::run().await?;
        }
        Cmd::Start => {
            Server::spawn_background().await?;
            println!("ok");
        }
        Cmd::Stop => {
            let resp = Server::try_stop().await?;
            print_and_exit(&resp)?;
        }
        Cmd::Status => {
            // Try to give a more actionable error message instead of a generic "not running".
            let paths = paths::Paths::new().ok();
            match Server::client_send(ClientRequest::Status).await {
                Ok(resp) => println!("{}", serde_json::to_string_pretty(&resp)?),
                Err(e) => {
                    if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
                        use std::io::ErrorKind::*;
                        if let Some(p) = paths {
                            let sock = &p.sock;
                            match ioe.kind() {
                                NotFound => {
                                    eprintln!(
                                        "status: agentd socket {:?} not found: {}.\n  \
hint: daemon 未在运行，或者 logs/agentd 被手动清理导致 sock 丢失；通常可用 `just agentd-start` 重新启动。\n  \
若你在 daemon 运行时删除了该目录，可能还残留旧的 loadlynx-agentd 进程，需要先杀掉进程再重启。",
                                        sock, ioe
                                    );
                                }
                                ConnectionRefused | BrokenPipe | ConnectionReset => {
                                    eprintln!(
                                        "status: 无法连接到 agentd {:?}: {} (连接被拒绝/中断)。\n  \
hint: daemon 正在启动、已崩溃或刚退出，可尝试 `just agentd-start` 重启。",
                                        sock, ioe
                                    );
                                }
                                PermissionDenied => {
                                    eprintln!(
                                        "status: 访问 agentd socket {:?} 权限不足: {}。\n  \
hint: 检查 logs/agentd 目录以及 sock 文件的所有者与权限。",
                                        sock, ioe
                                    );
                                }
                                _ => {
                                    eprintln!(
                                        "status: 查询 agentd 状态失败 (socket {:?}): {:#}",
                                        sock, e
                                    );
                                }
                            }
                        } else {
                            eprintln!("status: 查询 agentd 状态失败: {:#}", e);
                        }
                    } else {
                        eprintln!("status: 查询 agentd 状态失败: {:#}", e);
                    }
                }
            }
        }
        Cmd::SetPort { mcu, path } => {
            let mcu_kind: McuKind = mcu.clone().into();
            let p = match path {
                Some(p) => p,
                None => interactive_select_port(mcu_kind.clone()).await?,
            };
            let resp = client_send_with_autostart(ClientRequest::SetPort {
                mcu: mcu_kind,
                path: p,
            })
            .await?;
            print_and_exit(&resp)?;
        }
        Cmd::GetPort { mcu } => {
            let resp =
                client_send_with_autostart(ClientRequest::GetPort { mcu: mcu.into() }).await?;
            print_and_exit(&resp)?;
        }
        Cmd::ListPorts { mcu } => {
            let resp =
                client_send_with_autostart(ClientRequest::ListPorts { mcu: mcu.into() }).await?;
            print_and_exit(&resp)?;
        }
        Cmd::Flash { mcu, elf, after } => {
            let resp = client_send_with_autostart(ClientRequest::Flash {
                mcu: mcu.into(),
                elf,
                after: Some(after.into()),
            })
            .await?;
            print_and_exit(&resp)?;
        }
        Cmd::Reset { mcu } => {
            let resp =
                client_send_with_autostart(ClientRequest::Reset { mcu: mcu.into() }).await?;
            print_and_exit(&resp)?;
        }
        Cmd::Monitor {
            mcu,
            elf,
            reset,
            duration,
            lines,
        } => {
            let mcu_kind: McuKind = mcu.into();
            if reset {
                // Capture current latest log before reset (may be absent).
                let prev_path = match client_send_with_autostart(ClientRequest::Monitor {
                    mcu: mcu_kind.clone(),
                    elf: None,
                    duration: None,
                    lines: None,
                })
                .await
                {
                    Ok(resp) if resp.ok => resp
                        .payload
                        .get("path")
                        .and_then(|p| p.as_str())
                        .map(PathBuf::from),
                    _ => None,
                };

                // Reset via daemon (auto-start if needed).
                let reset_resp =
                    client_send_with_autostart(ClientRequest::Reset { mcu: mcu_kind.clone() })
                        .await?;
                if !reset_resp.ok {
                    return print_and_exit(&reset_resp);
                }

                // Wait for a new log file to appear.
                let timeout = TokioDuration::from_secs(5);
                let deadline = Instant::now() + timeout;
                let mut new_path: Option<PathBuf> = None;
                loop {
                    if Instant::now() >= deadline {
                        break;
                    }
                    let resp = client_send_with_autostart(ClientRequest::Monitor {
                        mcu: mcu_kind.clone(),
                        elf: None,
                        duration: None,
                        lines: None,
                    })
                    .await?;
                    if resp.ok {
                        if let Some(path_str) = resp.payload.get("path").and_then(|p| p.as_str())
                        {
                            let cand = PathBuf::from(path_str);
                            let is_new = match &prev_path {
                                Some(prev) => &cand != prev,
                                None => true,
                            };
                            if is_new {
                                new_path = Some(cand);
                                break;
                            }
                        }
                    }
                    sleep(TokioDuration::from_millis(100)).await;
                }

                let path = new_path.ok_or_else(|| {
                    anyhow!(
                        "no new session log for {:?} after reset (last: {:?})",
                        mcu_kind, prev_path
                    )
                })?;
                tail_file(path, duration, lines, true).await?;
            } else {
                let resp = client_send_with_autostart(ClientRequest::Monitor {
                    mcu: mcu_kind.clone(),
                    elf,
                    duration: if duration.as_millis() == 0 {
                        None
                    } else {
                        Some(duration.as_millis() as u64)
                    },
                    lines: if lines == 0 { None } else { Some(lines) },
                })
                .await?;
                if !resp.ok {
                    return print_and_exit(&resp);
                }
                if let Some(path) = resp.payload.get("path").and_then(|p| p.as_str()) {
                    tail_file(PathBuf::from(path), duration, lines, false).await?;
                } else {
                    eprintln!("monitor: daemon did not return a log path");
                }
            }
        }
        Cmd::Logs {
            mcu,
            since,
            until,
            tail,
            sessions,
        } => {
            let resp = client_send_with_autostart(ClientRequest::Logs {
                mcu: mcu.into(),
                since,
                until,
                tail,
                sessions,
            })
            .await?;
            print_and_exit(&resp)?;
        }
    }
    Ok(())
}

impl From<McuOpt> for McuKind {
    fn from(m: McuOpt) -> Self {
        match m {
            McuOpt::Digital => McuKind::Digital,
            McuOpt::Analog => McuKind::Analog,
        }
    }
}

impl From<LogsMcuOpt> for Option<McuKind> {
    fn from(m: LogsMcuOpt) -> Self {
        match m {
            LogsMcuOpt::Digital => Some(McuKind::Digital),
            LogsMcuOpt::Analog => Some(McuKind::Analog),
            LogsMcuOpt::All => None,
        }
    }
}

impl From<OptionAfter> for model::AfterPolicy {
    fn from(a: OptionAfter) -> Self {
        match a {
            OptionAfter::NoReset => model::AfterPolicy::NoReset,
            OptionAfter::HardReset => model::AfterPolicy::HardReset,
        }
    }
}

async fn tail_file(
    path: PathBuf,
    duration: std::time::Duration,
    lines: usize,
    from_start: bool,
) -> Result<()> {
    if !path.exists() {
        eprintln!("monitor: log file not found: {:?}", path);
        return Ok(());
    }
    let mut file = File::open(&path).await?;
    if from_start {
        file.seek(std::io::SeekFrom::Start(0)).await?;
    } else {
        file.seek(std::io::SeekFrom::End(0)).await?;
    }
    let mut reader = BufReader::new(file).lines();
    let deadline = if duration.as_millis() == 0 {
        None
    } else {
        Some(Instant::now() + TokioDuration::from_millis(duration.as_millis() as u64))
    };
    let mut remaining = if lines == 0 { None } else { Some(lines) };
    loop {
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return Ok(());
            }
        }
        match reader.next_line().await? {
            Some(l) => {
                let out = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&l) {
                    v.get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or(&l)
                        .to_string()
                } else {
                    l
                };
                println!("{}", out);
                if let Some(ref mut rem) = remaining {
                    if *rem == 0 {
                        return Ok(());
                    }
                    *rem -= 1;
                    if *rem == 0 {
                        return Ok(());
                    }
                }
            }
            None => {
                sleep(TokioDuration::from_millis(100)).await;
            }
        }
    }
}

async fn interactive_select_port(mcu: McuKind) -> Result<PathBuf> {
    match mcu {
        McuKind::Digital => {
            let ports = available_ports()?;
            if ports.is_empty() {
                bail!("未发现串口，请先接好设备再试");
            }
            // Prefer Espressif USB (VID 0x303A) and cu.* nodes on macOS for espflash friendliness.
            let filtered: Vec<&serialport::SerialPortInfo> = ports
                .iter()
                .filter(|p| matches!(p.port_type, SerialPortType::UsbPort(ref info) if info.vid == 0x303A))
                .filter(|p| p.port_name.contains("/cu."))
                .collect();

            if filtered.is_empty() {
                bail!("未找到 Espressif USB 串口（仅列 cu.*，VID=303A），可用 --path 显式指定");
            }
            let items: Vec<String> = filtered
                .iter()
                .map(|p| {
                    let extra = match &p.port_type {
                        SerialPortType::UsbPort(info) => format!(
                            "vid={:04x} pid={:04x} {:?}",
                            info.vid, info.pid, info.product
                        ),
                        _ => String::new(),
                    };
                    format!("{} ({extra})", p.port_name)
                })
                .collect();
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("选择 ESP32 串口 (上下箭头选择，回车确认)")
                .items(&items)
                .default(0)
                .interact()?;
            Ok(PathBuf::from(filtered[idx].port_name.clone()))
        }
        McuKind::Analog => {
            use tokio::process::Command;
            let out = Command::new("probe-rs").arg("list").output().await?;
            if !out.status.success() {
                bail!(
                    "probe-rs list 失败: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut entries: Vec<String> = stdout
                .lines()
                .filter(|l| l.trim_start().starts_with('['))
                .map(|l| l.trim().to_string())
                .collect();
            // Prefer STM32-friendly probes; drop ESP JTAG/WCH when possible.
            let preferred: Vec<String> = entries
                .iter()
                .filter(|l| {
                    l.contains("STLink")
                        || l.contains("ST-LINK")
                        || l.contains("CMSIS-DAP")
                        || l.contains("0483:3748")
                        || l.contains("0d28:0204")
                })
                .cloned()
                .collect();
            if !preferred.is_empty() {
                entries = preferred;
            }

            if entries.is_empty() {
                bail!("未发现调试探针，可用 --path 显式指定 probe-rs 标识");
            }
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("选择 STM32 调试探针 (上下箭头选择，回车确认)")
                .items(&entries)
                .default(0)
                .interact()?;
            let selected = &entries[idx];
            // line format: [1]: STLink V2 -- 0483:3748:SERIAL (ST-LINK)
            let id = selected
                .split("--")
                .nth(1)
                .map(|s| s.trim().split_whitespace().next().unwrap_or(s.trim()))
                .unwrap_or(selected);
            Ok(PathBuf::from(id))
        }
    }
}

// Try sending to daemon; if socket is missing or connection refused, auto-start and retry a few times.
async fn client_send_with_autostart(req: ClientRequest) -> Result<model::ClientResponse> {
    let paths = paths::Paths::new()?;
    let sock = paths.sock.clone();
    match Server::client_send(req.clone()).await {
        Ok(r) => Ok(r),
        Err(e) => {
            let (is_enoent, is_refused) = match e.downcast_ref::<std::io::Error>() {
                Some(ioe) => (
                    ioe.kind() == std::io::ErrorKind::NotFound,
                    matches!(
                        ioe.kind(),
                        std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::BrokenPipe
                            | std::io::ErrorKind::ConnectionReset
                    ),
                ),
                None => (false, false),
            };
            if !(is_enoent || is_refused) {
                return Err(e);
            }
            // auto-start daemon then retry once
            Server::spawn_background().await?;
            sleep(TokioDuration::from_millis(150)).await;
            let mut attempts = 0;
            loop {
                attempts += 1;
                match Server::client_send(req.clone()).await {
                    Ok(r) => return Ok(r),
                    Err(_e) if attempts < 5 => {
                        sleep(TokioDuration::from_millis(150)).await;
                        continue;
                    }
                    Err(e) => {
                        return Err(anyhow!(
                            "agentd not reachable at {:?}: {}. Try `just agentd-start` or check permissions (logs/agentd).",
                            sock,
                            e
                        ));
                    }
                }
            }
        }
    }
}
