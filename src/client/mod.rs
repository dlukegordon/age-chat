mod comms;
mod tui;

use std::fs::File;

use anyhow::Result;
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

    // Create a channel for coordinated shutdown
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(10);

    // Start communication with server
    let addr = format!("ws://{}", args.common.address);
    let mut comms = Comms::run(addr, shutdown_tx.clone(), shutdown_rx.resubscribe()).await?;

    // Run the TUI
    tui::run(&mut comms, shutdown_tx, shutdown_rx)?;

    // Shutdown
    comms.wait_shutdown().await?;
    info!("🛑 Client stopped");
    Ok(())
}
