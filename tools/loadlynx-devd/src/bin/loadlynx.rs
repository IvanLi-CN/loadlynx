use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use loadlynx_devd::{
    DEFAULT_DEVD_URL, TargetKind, list_digital_usb_port_candidates, write_default_digital_usb_port,
};
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::{env, io, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "loadlynx")]
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
    UsbPort {
        #[command(subcommand)]
        command: UsbPortCommand,
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

#[derive(Debug, Subcommand)]
enum UsbPortCommand {
    Set {
        #[arg(value_name = "TARGET_OR_PORT", num_args = 0..=2)]
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum UsbPortTarget {
    Digital,
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

fn api_url(base: &str, path: &str) -> Result<Url, Box<dyn std::error::Error + Send + Sync>> {
    let base_url = Url::parse(base)?;
    let inherited_query = base_url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let mut url = base_url;
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    url.set_path(path);
    url.set_query((!query.is_empty()).then_some(query));
    let existing_keys = url
        .query_pairs()
        .map(|(key, _)| key.into_owned())
        .collect::<Vec<_>>();
    if !inherited_query.is_empty() {
        url.query_pairs_mut().extend_pairs(
            inherited_query
                .iter()
                .filter(|(key, _)| !existing_keys.contains(key))
                .map(|(key, value)| (&**key, &**value)),
        );
    }
    Ok(url)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let client = Client::new();
    let payload =
        match cli.command {
            Command::UsbPort {
                command: UsbPortCommand::Set { args },
            } => {
                let (target, port) = resolve_usb_port_set_args(args)?;
                let repo_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match target {
                    UsbPortTarget::Digital => {
                        write_default_digital_usb_port(&repo_root, &port)?;
                        json!({"ok": true, "mcu": "digital", "default_usb_port": port})
                    }
                }
            }
            Command::Discover { mdns, lan_scan } => {
                let scan = client
                    .post(api_url(&cli.devd, "/api/v1/devices/scan")?)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?;
                json!({"mdns_requested": mdns, "lan_scan_requested": lan_scan, "devd": scan})
            }
            Command::Devices => {
                client
                    .get(api_url(&cli.devd, "/api/v1/devices")?)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?
            }
            Command::Status { url, device } => {
                if let Some(url) = url {
                    client
                        .get(api_url(&url, "/api/v1/status")?)
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Value>()
                        .await?
                } else if let Some(device) = device {
                    client
                        .get(api_url(&cli.devd, &format!("/api/v1/devices/{device}/status"))?)
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
            .post(api_url(&cli.devd, &format!("/api/v1/devices/{device}/flash"))?)
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
                .post(api_url(&cli.devd, &format!("/api/v1/devices/{device}/reset"))?)
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
                    .get(api_url(
                        &cli.devd,
                        &format!(
                            "/api/v1/devices/{device}/session?logs_limit={tail}&trace_limit={}",
                            tail * 2
                        ),
                    )?)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?
            }
            Command::Output { command } => match command {
                OutputCommand::Set { url, enable } => {
                    client
                        .post(api_url(&url, "/api/v1/cc")?)
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

fn resolve_usb_port_set_args(
    args: Vec<String>,
) -> Result<(UsbPortTarget, String), Box<dyn std::error::Error + Send + Sync>> {
    match args.as_slice() {
        [] => Ok((
            UsbPortTarget::Digital,
            choose_digital_usb_port_interactive()?,
        )),
        [single] if single == "digital" => Ok((
            UsbPortTarget::Digital,
            choose_digital_usb_port_interactive()?,
        )),
        [single] => Ok((UsbPortTarget::Digital, single.clone())),
        [target, port] if target == "digital" => Ok((UsbPortTarget::Digital, port.clone())),
        [target, _] => Err(format!("unsupported USB port target: {target}").into()),
        _ => Err("usb-port set accepts at most TARGET and PORT".into()),
    }
}

fn choose_digital_usb_port_interactive() -> io::Result<String> {
    let candidates = list_digital_usb_port_candidates();
    if candidates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No serial ports found. Connect the ESP32-S3 digital USB CDC device and retry.",
        ));
    }

    if candidates.len() == 1 {
        let candidate = &candidates[0];
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Use serial port {}?", candidate.display_name))
            .default(true)
            .interact()
            .map_err(io::Error::other)?;
        if confirmed {
            return Ok(candidate.port_path.clone());
        }
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "USB port selection cancelled",
        ));
    }

    let items = candidates
        .iter()
        .map(|candidate| {
            if candidate.recognized {
                format!("{} (recognized dev board)", candidate.display_name)
            } else {
                candidate.display_name.clone()
            }
        })
        .collect::<Vec<_>>();
    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select ESP32-S3 digital USB CDC serial port")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(io::Error::other)?
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::Interrupted, "USB port selection cancelled")
        })?;

    Ok(candidates[selected].port_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_port_set_args_accept_port_only() {
        let (target, port) =
            resolve_usb_port_set_args(vec!["/dev/cu.usbmodem212101".to_string()]).unwrap();
        assert!(matches!(target, UsbPortTarget::Digital));
        assert_eq!(port, "/dev/cu.usbmodem212101");
    }

    #[test]
    fn usb_port_set_args_accept_digital_and_port() {
        let (target, port) = resolve_usb_port_set_args(vec![
            "digital".to_string(),
            "/dev/cu.usbmodem212101".to_string(),
        ])
        .unwrap();
        assert!(matches!(target, UsbPortTarget::Digital));
        assert_eq!(port, "/dev/cu.usbmodem212101");
    }

    #[test]
    fn usb_port_set_args_reject_unknown_target() {
        let err = resolve_usb_port_set_args(vec![
            "analog".to_string(),
            "/dev/cu.usbmodem212101".to_string(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("unsupported USB port target"));
    }
}
