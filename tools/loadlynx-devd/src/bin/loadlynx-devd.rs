use clap::{Parser, Subcommand};
use loadlynx_devd::{
    DEFAULT_BIND, DEFAULT_IPC_IDLE_TIMEOUT_SECS, DevdConfig, IpcConfig, default_ipc_endpoint,
    serve, serve_ipc,
};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "loadlynx-devd")]
#[command(about = "LoadLynx local device daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        #[arg(long, alias = "bind", default_value_t = default_ipc_endpoint())]
        endpoint: String,
        #[arg(long, default_value_t = DEFAULT_IPC_IDLE_TIMEOUT_SECS)]
        idle_timeout_secs: u64,
    },
    BridgeHttp {
        #[arg(long, default_value = DEFAULT_BIND)]
        bind: SocketAddr,
        #[arg(long)]
        web_root: Option<PathBuf>,
        #[arg(long)]
        allow_dev_cors: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve {
            endpoint,
            idle_timeout_secs,
        } => {
            serve_ipc(IpcConfig::new(
                endpoint,
                std::time::Duration::from_secs(idle_timeout_secs),
            ))
            .await?
        }
        Command::BridgeHttp {
            bind,
            web_root,
            allow_dev_cors,
        } => serve(DevdConfig::new(bind, web_root, allow_dev_cors)).await?,
    }
    Ok(())
}
