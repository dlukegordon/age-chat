mod comms;

use anyhow::Result;
use tracing::info;

use crate::client::comms::Comms;
use crate::common::ClientMsg;
use crate::ClientArgs;

/// Entrance point to client from cli
pub async fn run(args: ClientArgs) -> Result<()> {
    info!("🏁 Client started");
    // connect_async requires the protocol prefix
    let addr = format!("ws://{}", args.common.address);

    // Start communication with server
    let mut comms = Comms::run(addr);

    comms
        .send_message(ClientMsg::send_new_note("hello".into()))
        .await?;
    let _msg = comms.receive_message().await?;

    comms.wait_shutdown().await?;
    info!("🛑 Client stopped");
    Ok(())
}
