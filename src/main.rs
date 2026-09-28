mod protocol;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use protocol::{read_message, write_message, Message, ProtocolVersion};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(name = "dropship", version, about = "Peer-to-peer file transfer over TCP")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Listen for an incoming peer connection.
    Listen {
        #[arg(short, long, default_value_t = 7878)]
        port: u16,
    },
    /// Connect to a peer and perform the initial protocol handshake.
    Send {
        path: PathBuf,
        address: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    match Cli::parse().command {
        Command::Listen { port } => listen(port).await?,
        Command::Send { path, address } => send(path, &address).await?,
    }

    Ok(())
}

async fn listen(port: u16) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!(port, "listening for peers");

    loop {
        let (stream, address) = listener.accept().await?;
        info!(%address, "accepted peer connection");
        tokio::spawn(async move {
            if let Err(error) = handle_peer(stream).await {
                warn!(%address, %error, "peer connection failed");
            }
        });
    }
}

async fn handle_peer(mut stream: TcpStream) -> Result<()> {
    let message = read_message(&mut stream).await?;
    info!(?message, "received protocol message");
    write_message(&mut stream, &Message::Hello(protocol::Hello {
        protocol_version: ProtocolVersion::CURRENT,
        peer_id: "listener".to_owned(),
    }))
    .await?;
    Ok(())
}

async fn send(path: PathBuf, address: &str) -> Result<()> {
    let mut stream = TcpStream::connect(address).await?;
    let message = Message::Hello(protocol::Hello {
        protocol_version: ProtocolVersion::CURRENT,
        peer_id: "sender".to_owned(),
    });
    write_message(&mut stream, &message).await?;
    info!(?path, address, "sent handshake");
    let response = read_message(&mut stream).await?;
    info!(?response, "received handshake response");
    Ok(())
}