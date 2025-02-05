use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tracing::info;

use crate::ServeArgs;

pub async fn run(args: ServeArgs) -> Result<()> {
    let address = args.common.address();
    let server = TcpListener::bind(&address).await?;
    info!("Server listening on {address}");

    let (mut tcp, _addr) = server.accept().await?;
    let mut buffer = [0u8; 16];
    loop {
        let n = tcp.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        let _ = tcp.write(&buffer[..n]).await?;
    }

    Ok(())
}
