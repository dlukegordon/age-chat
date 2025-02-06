mod comms;

use anyhow::Result;

use crate::ClientArgs;

/// Entrance point to client from cli
pub async fn run(args: ClientArgs) -> Result<()> {
    // connect_async requires the protocol prefix
    let addr = format!("ws://{}", args.common.address);
    comms::talk_server(&addr).await?;

    Ok(())
}
