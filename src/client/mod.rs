mod comms;
mod tui;

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::info;

use crate::client::comms::Comms;
use crate::ClientArgs;

/// Entrance point to client from cli
pub async fn run(args: ClientArgs) -> Result<()> {
    info!("🏁 Client started");

    // Create a channel for coordinated shutdown
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(10);

    // Start communication with server
    let addr = format!("ws://{}", args.common.address);
    let comms = Comms::run(addr, shutdown_tx.clone(), shutdown_rx.resubscribe()).await?;

    // comms
    //     .send_message(ClientMsg::send_new_note("hello".into()))
    //     .await?;
    // let _msg = comms.receive_message().await?;
    tui::run(shutdown_tx, shutdown_rx)?;

    // Shutdown
    comms.wait_shutdown().await?;
    info!("🛑 Client stopped");
    Ok(())
}
