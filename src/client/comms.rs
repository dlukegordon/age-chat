use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::str::FromStr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::signal;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tracing::{error, info};

use crate::common::{ClientMsg, ServerMsg};

const CHANNEL_BUFFER_SIZE: usize = 1000;

/// Manages communication with the server
pub struct Comms {
    incoming_rx: Receiver<ServerMsg>,
    outgoing_tx: Sender<ClientMsg>,
    task_handle: JoinHandle<()>,
}

impl Comms {
    /// Start the background server communication task
    pub fn run(addr: String) -> Self {
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<ClientMsg>(CHANNEL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel::<ServerMsg>(CHANNEL_BUFFER_SIZE);

        let task_handle = tokio::spawn(async move {
            let res = talk_server(&addr, &mut outgoing_rx, incoming_tx).await;
            if let Err(e) = res {
                error!("Error talking to server: {e}");
            }
        });

        Comms {
            incoming_rx,
            outgoing_tx,
            task_handle,
        }
    }

    /// Send a message to the server
    pub async fn send_message(&self, message: ClientMsg) -> Result<()> {
        self.outgoing_tx.send(message).await?;
        Ok(())
    }

    /// Receive a message from the server
    pub async fn receive_message(&mut self) -> Result<ServerMsg> {
        self.incoming_rx
            .recv()
            .await
            .ok_or(anyhow!("Incoming message channel is closed"))
    }

    /// Wait for the communication task to end
    pub async fn wait_shutdown(self) -> Result<()> {
        self.task_handle.await?;
        Ok(())
    }
}

/// Connect to the server and talk to it
async fn talk_server(
    addr: &str,
    outgoing_rx: &mut Receiver<ClientMsg>,
    incoming_tx: Sender<ServerMsg>,
) -> Result<()> {
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
    let res = talk_server_socket(outgoing_rx, incoming_tx, &mut socket).await;
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
async fn talk_server_socket<T>(
    outgoing_rx: &mut Receiver<ClientMsg>,
    incoming_tx: Sender<ServerMsg>,
    socket: &mut WebSocketStream<T>,
) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let (mut write, mut read) = socket.split();

    loop {
        tokio::select! {
            // Send outgoing messages from channel to server
            client_msg_opt = outgoing_rx.recv() => {
                let msg = client_msg_opt.ok_or(anyhow!("Outgoing message channel closed"))?;
                info!("📤 Sending message: {msg:?}");
                let ws_msg = msg.to_ws_msg();
                write.send(ws_msg).await.context("Error sending WS message to the server")?
            }

            // Receive incoming messages from server to channel
            ws_msg_res_opt = read.next() => {
                let ws_msg = ws_msg_res_opt.ok_or(anyhow!("Connection to server closed"))??;
                if let Message::Text(payload) = ws_msg {
                    let msg = ServerMsg::from_str(&payload).context("Error deserializing ServerMsg")?;
                    info!("📥 Received message: {msg:?}");
                    incoming_tx.send(msg).await.context("incoming message channel is closed")?;
                }
            }

            // Shutdown
            res = signal::ctrl_c() => {
                info!("⛔ Received ctrl-c, shutting down");
                return res.context("Error listening for shutdown signal")
            }

        }
    }
}
