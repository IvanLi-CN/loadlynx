use clap::{Parser, Subcommand, ValueEnum};
use loadlynx_devd::{DEFAULT_DEVD_URL, TargetKind};
use reqwest::Client;
use serde_json::{Value, json};

#[derive(Debug, Parser)]
#[command(name = "loadlynxctl")]
#[command(about = "LoadLynx LAN/USB/devd control CLI")]
struct Cli {
    #[arg(long, default_value = DEFAULT_DEVD_URL)]
    devd: String,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Discover {
        #[arg(long)]
        mdns: bool,
        #[arg(long)]
        lan_scan: bool,
    },
    Devices,
    Status {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        device: Option<String>,
    },
    Flash {
        target: BoardTarget,
        #[arg(long)]
        device: String,
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long)]
        lease_id: Option<String>,
        #[arg(long, default_value_t = true)]
        dry_run: bool,
    },
    Reset {
        target: BoardTarget,
        #[arg(long)]
        device: String,
        #[arg(long)]
        lease_id: Option<String>,
        #[arg(long, default_value_t = true)]
        dry_run: bool,
    },
    Monitor {
        target: BoardTarget,
        #[arg(long)]
        device: String,
        #[arg(long, default_value_t = 200)]
        tail: usize,
    },
    Output {
        #[command(subcommand)]
        command: OutputCommand,
    },
}

#[derive(Debug, Subcommand)]
enum OutputCommand {
    Set {
        #[arg(long)]
        url: String,
        #[arg(long)]
        enable: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum BoardTarget {
    Digital,
    Analog,
}

impl BoardTarget {
    fn kind(&self) -> TargetKind {
        match self {
            Self::Digital => TargetKind::DigitalEsp32s3,
            Self::Analog => TargetKind::AnalogStm32g431,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let client = Client::new();
    let payload =
        match cli.command {
            Command::Discover { mdns, lan_scan } => {
                let scan = client
                    .post(format!("{}/api/v1/devices/scan", cli.devd))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?;
                json!({"mdns_requested": mdns, "lan_scan_requested": lan_scan, "devd": scan})
            }
            Command::Devices => {
                client
                    .get(format!("{}/api/v1/devices", cli.devd))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?
            }
            Command::Status { url, device } => {
                if let Some(url) = url {
                    client
                        .get(format!("{}/api/v1/status", url.trim_end_matches('/')))
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                } else if let Some(device) = device {
                    client
                        .get(format!("{}/api/v1/devices/{device}/status", cli.devd))
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                } else {
                    return Err("status requires --url or --device".into());
                }
            }
            Command::Flash {
            target,
            device,
            artifact,
            lease_id,
            dry_run,
        } => client
            .post(format!("{}/api/v1/devices/{device}/flash", cli.devd))
            .json(
                &json!({"target": target.kind(), "artifact_id": artifact, "lease_id": lease_id, "dry_run": dry_run}),
            )
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await?,
            Command::Reset {
            target,
            device,
            lease_id,
            dry_run,
        } => {
            client
                .post(format!("{}/api/v1/devices/{device}/reset", cli.devd))
                .json(&json!({"target": target.kind(), "lease_id": lease_id, "dry_run": dry_run}))
                .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?
            }
            Command::Monitor {
                target: _,
                device,
                tail,
            } => {
                client
                    .get(format!(
                        "{}/api/v1/devices/{device}/session?logs_limit={tail}&trace_limit={}",
                        cli.devd,
                        tail * 2
                    ))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?
            }
            Command::Output { command } => match command {
                OutputCommand::Set { url, enable } => {
                    client
                        .post(format!("{}/api/v1/cc", url.trim_end_matches('/')))
                        .json(&json!({"enable": enable}))
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                }
            },
        };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    }
    Ok(())
}
