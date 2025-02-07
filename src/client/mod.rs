mod comms;

use anyhow::Result;
use tracing::info;

use crate::client::comms::Comms;
use crate::common::ClientMsg;
use crate::ClientArgs;

/// Entrance point to client from cli
pub async fn run(args: ClientArgs) -> Result<()> {
    info!("🏁 Client started");

    // Start communication with server
    let addr = format!("ws://{}", args.common.address);
    let mut comms = Comms::run(addr).await?;

    comms
        .send_message(ClientMsg::send_new_note("hello".into()))
        .await?;
    let _msg = comms.receive_message().await?;

    // Shutdown
    comms.wait_shutdown().await?;
    info!("🛑 Client stopped");
    Ok(())
}
