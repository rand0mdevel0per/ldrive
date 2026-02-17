use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod cmd_send;
mod cmd_recv;
mod cmd_serve;
mod cmd_publish;
mod cmd_fetch;

#[derive(Parser)]
#[command(name = "ldrive-node", about = "LDrive distributed storage node")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Send a file directly to a remote peer (no DHT)
    Send {
        /// Path to the file to send
        file: PathBuf,
        /// Remote peer address (host:port)
        peer: SocketAddr,
    },
    /// Receive files directly from a remote peer (no DHT)
    Recv {
        /// Address to listen on
        #[arg(long, default_value = "0.0.0.0:4433")]
        listen: SocketAddr,
        /// Output directory for received files
        #[arg(long, default_value = ".")]
        output: PathBuf,
    },
    /// Run as a persistent storage node with DHT
    Serve {
        /// Address to listen on
        #[arg(long, default_value = "0.0.0.0:4433")]
        listen: SocketAddr,
        /// Storage directory
        #[arg(long, default_value = "./ldrive-data")]
        storage_path: PathBuf,
        /// Storage quota in bytes (default 10 GB)
        #[arg(long, default_value = "10737418240")]
        quota: u64,
        /// Bootstrap peer addresses (comma-separated)
        #[arg(long, value_delimiter = ',')]
        bootstrap: Vec<SocketAddr>,
    },
    /// Publish a file to the DHT network
    Publish {
        /// Path to the file to publish
        file: PathBuf,
        /// Address to listen on (for chunk serving)
        #[arg(long, default_value = "0.0.0.0:4434")]
        listen: SocketAddr,
        /// Storage directory for local chunks
        #[arg(long, default_value = "./ldrive-data")]
        storage_path: PathBuf,
        /// Bootstrap peer addresses
        #[arg(long, value_delimiter = ',')]
        bootstrap: Vec<SocketAddr>,
    },
    /// Fetch a file from the DHT network by file hash
    Fetch {
        /// File hash (hex-encoded BLAKE3)
        hash: String,
        /// Output path
        #[arg(long, default_value = ".")]
        output: PathBuf,
        /// Bootstrap peer addresses
        #[arg(long, value_delimiter = ',')]
        bootstrap: Vec<SocketAddr>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Send { file, peer } => cmd_send::run(file, peer).await,
        Command::Recv { listen, output } => cmd_recv::run(listen, output).await,
        Command::Serve {
            listen,
            storage_path,
            quota,
            bootstrap,
        } => cmd_serve::run(listen, storage_path, quota, bootstrap).await,
        Command::Publish {
            file,
            listen,
            storage_path,
            bootstrap,
        } => cmd_publish::run(file, listen, storage_path, bootstrap).await,
        Command::Fetch {
            hash,
            output,
            bootstrap,
        } => cmd_fetch::run(hash, output, bootstrap).await,
    }
}
