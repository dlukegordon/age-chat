mod comms;
mod tui;

use std::fs::File;
use std::str::FromStr;

use age::x25519::{Identity, Recipient};
use anyhow::{anyhow, Result};
use tokio::sync::broadcast;
use tracing::info;

use crate::client::comms::Comms;
use crate::ClientArgs;

const LOG_PATH: &str = "client.log";

/// Entrance point to client from cli
pub async fn run(args: ClientArgs) -> Result<()> {
    // Logging
    let file = File::create(LOG_PATH)?;
    tracing_subscriber::fmt().with_writer(file).init();
    info!("🏁 Client started");

    // Load the key file
    let key_file = std::fs::read_to_string(args.key_file)?
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<&str>>()
        .join("\n");
    let key = Identity::from_str(&key_file).map_err(|e| anyhow!(e))?;
    let recipient = Recipient::from_str(&args.recipient).map_err(|e| anyhow!(e))?;
    info!("🔑 Key file loaded");

    // Create a channel for coordinated shutdown
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

    // Start communication with server
    let addr = format!("ws://{}", args.common.address);
    let mut comms = Comms::run(addr, shutdown_tx.clone(), shutdown_rx.resubscribe()).await?;

    // Run the TUI
    tui::run(&mut comms, key, recipient, shutdown_tx, shutdown_rx)?;

    // Shutdown
    comms.wait_shutdown().await?;
    info!("🛑 Client stopped");
    Ok(())
}
