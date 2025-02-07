mod comms;

use anyhow::Result;
use tracing::info;

use crate::ServerArgs;

/// Entrance point to server from cli
pub async fn run(args: ServerArgs) -> Result<()> {
    info!("🏁 Server started");
    comms::serve(&args.common.address).await?;
    info!("🛑 Server stopped");
    Ok(())
}
