mod comms;

use anyhow::Result;

use crate::ServerArgs;

/// Entrance point to server from cli
pub async fn run(args: ServerArgs) -> Result<()> {
    comms::serve(&args.common.address).await?;
    Ok(())
}
