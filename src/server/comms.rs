use anyhow::{anyhow, Context, Result};
use futures_util::{future::join_all, SinkExt, StreamExt};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    signal,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Utf8Bytes},
    WebSocketStream,
};
use tracing::{error, info};

use crate::common::{ClientMsg, RecNote, SendNote, ServerMsg};

/// Run the server
pub async fn serve(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!("📡 Server listening on {addr}");

    let mut task_handles = vec![];
    loop {
        tokio::select! {
            // Serve connections
            accept_res = listener.accept() => {
                let (stream, _addr) = accept_res.context("Error accepting tcp connection")?;
                let handle = tokio::spawn(async move {
                    let res = serve_client_tcp_conn(stream).await;
                    if let Err(e) = res {
                        error!("Error serving tcp connection: {e}");
                    }
                });
                task_handles.push(handle);
            }

            // Shutdown
            res = signal::ctrl_c() => {
                res.context("Error listening for shutdown signal")?;
                info!("⛔ Received ctrl-c in serve, shutting down");
                // Wait for connections to close
                join_all(task_handles).await;
                return Ok(());
            }
        }
    }
}

/// Upgrade client tcp connection to websocket and serve
async fn serve_client_tcp_conn(tcp_stream: TcpStream) -> Result<()> {
    // Open ws connection to client
    let peer_addr = tcp_stream.peer_addr()?;
    let mut socket = accept_async(tcp_stream).await?;
    info!("🔗 Connected to client: {peer_addr}");

    // Serve the client
    let res = serve_client_ws_conn(peer_addr, &mut socket).await;
    if let Err(e) = res {
        error!("Error serving WS connection {peer_addr}: {e}");
    }

    // Close connection to client. It's fine if it errors out.
    _ = socket.close(None).await;
    info!("⛓️‍💥 Disconnected from client: {peer_addr}");
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
    loop {
        tokio::select! {
            // Handle incoming WS messages from client
            ws_msg_res_opt = socket.next() => {
                let ws_msg = ws_msg_res_opt.ok_or(anyhow!("Connection to server closed"))??;
                info!("Received WS message from {peer_addr}: {ws_msg:?}");

                match ws_msg {
                    Message::Text(payload) => {
                        handle_client_ws_text_msg(socket, peer_addr, payload)
                            .await
                            .context("Error handling client text WS text message")?;
                    }

                    Message::Close(_frame) => {
                        info!("👋 Received WS close message from {peer_addr}, disconnecting");
                        return Ok(());
                    },
                    Message::Binary(_payload) => error!("Server does not support binary messages"),
                    Message::Frame(_frame) => error!("Server does not support frame messages"),
                    // tokio_tungstenite automatically handles ping/pong
                    _ => {}
                }
            }

            // Shutdown
            res = signal::ctrl_c() => {
                res.context("Error listening for shutdown signal")?;
                info!("⛔ Received ctrl-c in serve_client_ws_conn, shutting down");
                return Ok(());
            }
        }
    }
}

/// Handle WS text messages from the client
async fn handle_client_ws_text_msg<T>(
    socket: &mut WebSocketStream<T>,
    peer_addr: SocketAddr,
    payload: Utf8Bytes,
) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let msg = ClientMsg::from_str(&payload)?;
    info!("📥 Received message from {peer_addr}: {msg}");

    match msg {
        ClientMsg::SendNote(SendNote { note }) => {
            info!("✉️ Client {peer_addr} sent new note, echoing back");
            socket
                .send(ServerMsg::RecNote(RecNote { note }).to_ws_msg())
                .await?;
        }
    }

    Ok(())
}
