use anyhow::Result;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Utf8Bytes},
    WebSocketStream,
};
use tracing::{error, info};

use crate::{common::ClientWsMsg, ServerArgs};

/// Entrance point to server from cli
pub async fn run(args: ServerArgs) -> Result<()> {
    serve(&args.common.address).await?;
    Ok(())
}

/// Run the server
pub async fn serve(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!("📡 Server listening on {addr}");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    let res = serve_client_tcp_conn(stream).await;
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
async fn serve_client_tcp_conn(tcp_stream: TcpStream) -> Result<()> {
    let peer_addr = tcp_stream.peer_addr()?;
    let mut socket = accept_async(tcp_stream).await?;
    info!("🔗 Connected: {peer_addr}");

    let res = serve_client_ws_conn(peer_addr, &mut socket).await;
    if let Err(e) = res {
        error!("Error serving WS connection {peer_addr}: {e}");
    }

    info!("⛓️‍💥 Disconnected: {peer_addr}");
    Ok(())
}

/// Serve client websocket connection
async fn serve_client_ws_conn<T>(
    peer_addr: SocketAddr,
    socket: &mut WebSocketStream<T>,
) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    while let Some(ws_msg_res) = socket.next().await {
        let ws_msg = match ws_msg_res {
            Ok(ret) => ret,
            Err(e) => {
                error!("Error receiving next WS message from {peer_addr}: {e}");
                break;
            }
        };

        info!("📥 Received WS message from {peer_addr}: {ws_msg:?}");
        match ws_msg {
            Message::Text(payload) => {
                let res = handle_client_text_msg(peer_addr, payload).await;
                if let Err(e) = res {
                    error!("Error handling client WS message from {peer_addr}: {e}");
                }
            }

            Message::Binary(_payload) => error!("Server does not support binary messages"),
            Message::Frame(_frame) => error!("Server does not support frame messages"),

            // tokio_tungstenite automatically handles close handshakes and ping/pong
            _ => {}
        }
    }

    Ok(())
}

async fn handle_client_text_msg(peer_addr: SocketAddr, payload: Utf8Bytes) -> Result<()> {
    let msg = ClientWsMsg::from_str(&payload)?;

    match msg {
        ClientWsMsg::SendNewNote(note) => {
            info!("✉️ Client {peer_addr} sent new note:\n{note:?}");
        }
    }

    Ok(())
}
