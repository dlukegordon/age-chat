use std::net::SocketAddr;

use anyhow::Result;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Utf8Bytes},
    WebSocketStream,
};
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
                    let res = serve_tcp_conn(stream).await;
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
async fn serve_tcp_conn(tcp_stream: TcpStream) -> Result<()> {
    let peer_addr = tcp_stream.peer_addr()?;
    let ws_stream = accept_async(tcp_stream).await?;
    info!("🔗 Connected: {peer_addr}");

    let res = serve_ws_conn(peer_addr, ws_stream).await;
    if let Err(e) = res {
        error!("Error serving ws connection {peer_addr}: {e}");
    }

    info!("👋 Disconnected: {peer_addr}");
    Ok(())
}

/// Serve client websocket connection
async fn serve_ws_conn(peer_addr: SocketAddr, ws_stream: WebSocketStream<TcpStream>) -> Result<()> {
    let (mut write, mut read) = ws_stream.split();

    while let Some(ws_msg) = read.next().await {
        match ws_msg? {
            Message::Text(payload) => {
                info!("Received new ws text message from {peer_addr}");
                handle_ws_text_msg(payload, &mut write).await?;
            }
            // TODO: Handle ping/pong, close, etc?
            _ => {
                error!("Received invalid non-text ws message from {peer_addr}");
            }
        }
    }

    Ok(())
}

/// Handle a received websocket text message
async fn handle_ws_text_msg(
    payload: Utf8Bytes,
    write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
) -> Result<()> {
    write.send(Message::Text(payload)).await?;
    Ok(())
}
