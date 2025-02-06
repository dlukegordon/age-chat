use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tracing::{error, info};

pub async fn talk_server(addr: &str) -> Result<()> {
    // Open connection to server
    let (mut socket, _) = match connect_async(addr).await {
        Ok(ret) => ret,
        Err(e) => {
            error!("❌ Cannot connect to {addr}: {e}");
            // Just returning Ok for ux
            return Ok(());
        }
    };
    info!("🔗 Connected to server: {addr}");

    // Talk to the server over the socket
    let res = talk_server_socket(&mut socket).await;
    if let Err(e) = res {
        error!("Error talking to the server {addr} over the WS connection: {e}");
    }

    // Close connection to server
    let res = socket.close(None).await;
    if let Err(e) = res {
        error!("Error closing the server {addr} WS connection: {e}");
    }
    info!("⛓️‍💥 Disconnected from server: {addr}");
    Ok(())
}

/// Talk to the server over the websocket connection
async fn talk_server_socket<T>(socket: &mut WebSocketStream<T>) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    socket.send(Message::Ping("test".into())).await?;
    while let Some(ws_msg_res) = socket.next().await {
        let ws_msg = match ws_msg_res {
            Ok(ret) => ret,
            Err(e) => {
                error!("Error receiving next WS message: {e}");
                break;
            }
        };
        info!("📥 Received WS message: {ws_msg:?}");
    }
    Ok(())
}
