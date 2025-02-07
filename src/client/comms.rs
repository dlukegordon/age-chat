use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::str::FromStr;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
    },
    task::JoinHandle,
};
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
    /// Connect to the server and start the background server communication task. This will allow
    /// us to communicate with the server through channels. Will not finish awaiting until the server
    /// is connected.
    pub async fn run(
        addr: String,
        shutdown_tx: broadcast::Sender<()>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<Self> {
        // Channel to send messages to server
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<ClientMsg>(CHANNEL_BUFFER_SIZE);
        // Channel to receive messages from server
        let (incoming_tx, incoming_rx) = mpsc::channel::<ServerMsg>(CHANNEL_BUFFER_SIZE);

        // Open connection to server
        let (mut socket, _) = connect_async(&addr)
            .await
            .context(format!("Cannot connect to {addr}"))?;
        info!("🔗 Connected to server: {addr}");

        // Start the background server communication task
        let task_handle = tokio::spawn(async move {
            // Talk to the server over the socket
            let res = talk_server_socket(
                &mut outgoing_rx,
                incoming_tx,
                shutdown_tx,
                shutdown_rx,
                &mut socket,
            )
            .await;
            if let Err(e) = res {
                error!("Error talking to the server {addr}: {e}");
            }

            // Close connection to server. It's fine if it errors out.
            _ = socket.close(None).await;
            info!("⛓️‍💥 Disconnected from server: {addr}");
        });

        Ok(Comms {
            incoming_rx,
            outgoing_tx,
            task_handle,
        })
    }

    /// Send a message to the server
    pub async fn send_msg(&self, message: ClientMsg) -> Result<()> {
        self.outgoing_tx.send(message).await?;
        Ok(())
    }

    /// Receive a message from the server
    pub async fn recv_msg(&mut self) -> Result<ServerMsg> {
        self.incoming_rx
            .recv()
            .await
            .ok_or(anyhow!("Incoming message channel is closed"))
    }

    /// Send a message to the server without blocking
    pub fn try_send_msg(&self, message: ClientMsg) -> Result<()> {
        Ok(self.outgoing_tx.try_send(message)?)
    }

    /// Receive a message from the server without blocking
    pub fn try_recv_msg(&mut self) -> Result<ServerMsg> {
        Ok(self.incoming_rx.try_recv()?)
    }

    /// Wait for the communication task to end
    pub async fn wait_shutdown(self) -> Result<()> {
        self.task_handle.await?;
        Ok(())
    }
}

/// Talk to the server over the websocket connection, simultaneously sending messages from the
/// outgoing channel and putting received messages into the incoming channel.
async fn talk_server_socket<T>(
    outgoing_rx: &mut Receiver<ClientMsg>,
    incoming_tx: Sender<ServerMsg>,
    shutdown_tx: broadcast::Sender<()>,
    mut shutdown_rx: broadcast::Receiver<()>,
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
                match ws_msg {
                    Message::Text(payload) => {
                        let msg = ServerMsg::from_str(&payload).context("Error deserializing ServerMsg")?;
                        info!("📥 Received message: {msg:?}");
                        incoming_tx.send(msg).await.context("Incoming message channel is closed")?;
                    }
                    Message::Close(_frame) => {
                        info!("👋 Received WS close message from server, disconnecting");
                        shutdown_tx.send(())?;
                        return Ok(());
                    },
                    _ => {},
                }
            }

            // Shutdown
            res = shutdown_rx.recv() => {
                res.context("Error listening for shutdown signal")?;
                info!("⛔ Received shutdown signal");
                return Ok(());
            }
        }
    }
}
