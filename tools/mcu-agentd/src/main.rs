mod config;
mod model;
mod paths;
mod port_cache;
mod process;
mod server;
mod timefmt;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use model::{ClientRequest, McuKind};
use server::Server;
use std::path::PathBuf;

/// MCU agentd – single-instance helper for LoadLynx boards (ESP32-S3 + STM32G431).
#[derive(Parser, Debug)]
#[command(name = "mcu-agentd", version)]
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
        path: PathBuf,
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
        /// ELF path; if omitted, auto-builds default target.
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
        /// Optional ELF path; if missing, auto-builds default.
        elf: Option<PathBuf>,
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
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::Status => match Server::client_send(ClientRequest::Status).await {
            Ok(resp) => println!("{}", serde_json::to_string_pretty(&resp)?),
            Err(e) => {
                eprintln!("status: not running ({e})");
            }
        },
        Cmd::SetPort { mcu, path } => {
            let resp = Server::client_send(ClientRequest::SetPort {
                mcu: mcu.into(),
                path,
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::GetPort { mcu } => {
            let resp = Server::client_send(ClientRequest::GetPort { mcu: mcu.into() }).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::ListPorts { mcu } => {
            let resp = Server::client_send(ClientRequest::ListPorts { mcu: mcu.into() }).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::Flash { mcu, elf, after } => {
            let resp = Server::client_send(ClientRequest::Flash {
                mcu: mcu.into(),
                elf,
                after: Some(after.into()),
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::Reset { mcu } => {
            let resp = Server::client_send(ClientRequest::Reset { mcu: mcu.into() }).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::Monitor {
            mcu,
            elf,
            duration,
            lines,
        } => {
            let resp = Server::client_send(ClientRequest::Monitor {
                mcu: mcu.into(),
                elf,
                duration: if duration.as_millis() == 0 {
                    None
                } else {
                    Some(duration.as_millis() as u64)
                },
                lines: if lines == 0 { None } else { Some(lines) },
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Cmd::Logs {
            mcu,
            since,
            until,
            tail,
            sessions,
        } => {
            let resp = Server::client_send(ClientRequest::Logs {
                mcu: mcu.into(),
                since,
                until,
                tail,
                sessions,
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
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
