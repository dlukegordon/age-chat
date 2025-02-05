use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream};
use tracing::{error, info};

use crate::ServeArgs;

/// Entrance point to server from cli
pub async fn run(args: ServeArgs) -> Result<()> {
    let addr = args.common.address;
    let listener = TcpListener::bind(&addr).await?;
    info!("📡 Server listening on {addr}");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    let res = serve_tcp_connection(stream).await;
                    if let Err(e) = res {
                        error!("Error serving tcp connection: {e}");
                    }
                });
            }
            Err(e) => {
                error!("Error accepting tcp connection: {e}");
                break;
            }
        }
    }

    info!("🛑 Server stopped");
    Ok(())
}

/// Upgrade client tcp connection to websocket and serve
async fn serve_tcp_connection(tcp_stream: TcpStream) -> Result<()> {
    let peer_addr = tcp_stream.peer_addr()?;
    let ws_stream = accept_async(tcp_stream).await?;
    info!("🔗 Connected: {peer_addr}");

    let res = serve_ws_connection(ws_stream).await;
    if let Err(e) = res {
        error!("Error serving ws connection {peer_addr}: {e}");
    }

    info!("👋 Disconnected: {peer_addr}");
    Ok(())
}

/// Serve client websocket connection
async fn serve_ws_connection(ws_stream: WebSocketStream<TcpStream>) -> Result<()> {
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        write.send(msg?).await?;
    }

    Ok(())
}
