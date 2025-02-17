use age::x25519::Recipient;
use anyhow::{anyhow, Context, Result};
use futures_util::{future::join_all, SinkExt, StreamExt};
use rand::RngCore;
use std::collections::HashMap;
use std::str::FromStr;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::{
    net::{TcpListener, TcpStream},
    signal,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Utf8Bytes},
    WebSocketStream,
};
use tracing::{error, info};

use crate::common::{Auth, ClientMsg, Note, ServerMsg, CHANNEL_BUFFER_SIZE};

type UserConns = Arc<RwLock<HashMap<String, Sender<Note>>>>;

/// Run the server
pub async fn serve(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!("📡 Server listening on {addr}");

    // Create map of usernames to channels for sending notes
    let user_conns: UserConns = Arc::new(RwLock::new(HashMap::new()));

    let mut task_handles = vec![];
    loop {
        tokio::select! {
            // Serve connections
            accept_res = listener.accept() => {
                let (stream, _addr) = accept_res.context("Error accepting tcp connection")?;
                let user_conns = Arc::clone(&user_conns);
                let handle = tokio::spawn(async move {
                    let conn = match Connection::new(stream, user_conns).await {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!("Error creating connection: {e}");
                            return;
                        }
                    };

                    let res = conn.serve().await;
                    if let Err(e) = res {
                        error!("Error serving connection: {e}");
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

struct Connection {
    socket: WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
    user_conns: UserConns,
    note_tx: Sender<Note>,
    note_rx: Receiver<Note>,
    // Track authentication state
    pub_key: Option<String>,
    auth_secret: Option<String>,
}

impl Connection {
    async fn new(tcp_stream: TcpStream, user_conns: UserConns) -> Result<Self> {
        // Open WS connection to client
        let peer_addr = tcp_stream.peer_addr()?;
        let socket = accept_async(tcp_stream).await?;
        info!("🔗 Connected to client: {peer_addr}");

        // Channel to send notes through
        let (note_tx, note_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        Ok(Self {
            socket,
            peer_addr,
            user_conns,
            note_tx,
            note_rx,
            pub_key: None,
            auth_secret: None,
        })
    }

    /// Upgrade client tcp connection to websocket and serve
    async fn serve(mut self) -> Result<()> {
        // Serve the client
        let res = self.serve_client_ws_conn().await;
        if let Err(e) = res {
            error!("Error serving WS connection {}: {e}", self.peer_addr);
        }

        // Clean up user_conns
        if let Some(username) = self.pub_key {
            let mut user_conns_write = self.user_conns.write().await;
            user_conns_write.remove(&username);
        }

        // Close connection to client. It's fine if it errors out.
        _ = self.socket.close(None).await;
        info!("⛓️‍💥 Disconnected from client: {}", self.peer_addr);
        Ok(())
    }

    /// Serve client websocket connection
    async fn serve_client_ws_conn(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                // Handle incoming WS messages from client
                ws_msg_res_opt = self.socket.next() => {
                    let ws_msg = ws_msg_res_opt.ok_or(anyhow!("Connection to server closed"))??;
                    info!("Received WS message from {}: {ws_msg:?}", self.peer_addr);

                    match ws_msg {
                        Message::Text(payload) => {
                            self.handle_client_ws_text_msg(payload).await?
                        }

                        Message::Close(_frame) => {
                            info!("👋 Received WS close message from {}, disconnecting", self.peer_addr);
                            return Ok(());
                        },
                        Message::Binary(_payload) => error!("Server does not support binary messages"),
                        Message::Frame(_frame) => error!("Server does not support frame messages"),
                        // tokio_tungstenite automatically handles ping/pong
                        _ => {}
                    }
                }

                // Send notes from channel
                note_opt = self.note_rx.recv() =>  {
                    let note = note_opt.ok_or(anyhow!("Note channel for {} closed", self.peer_addr))?;
                    info!(
                        "✉️ Client {} receiving note from {} to {}",
                        self.peer_addr, note.from, note.to
                    );
                    self.socket
                        .send(ServerMsg::RecNote(note).to_ws_msg())
                        .await?;
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
    async fn handle_client_ws_text_msg(&mut self, payload: Utf8Bytes) -> Result<()> {
        let msg = ClientMsg::from_str(&payload)?;
        info!("📥 Received message from {}: {msg}", self.peer_addr);

        match msg {
            ClientMsg::AuthReq(auth) => self.handle_auth_req(auth).await?,
            ClientMsg::AuthPlaintext(auth) => self.handle_auth_plaintext(auth).await?,
            ClientMsg::SendNote(note) => self.handle_send_note(note).await?,
        }
        Ok(())
    }

    /// Handle the client requesting to authenticate
    async fn handle_auth_req(&mut self, auth: Auth) -> Result<()> {
        info!(
            "✍️ Client {} attempting to authenticate as {}",
            self.peer_addr, auth.pub_key
        );

        // Generate random secret and encrypt to client
        let mut bytes = [0u8; 64];
        rand::rng().fill_bytes(&mut bytes);
        let secret = hex::encode(bytes);
        self.auth_secret = Some(secret.clone());
        let recipient = Recipient::from_str(&auth.pub_key).map_err(|e| anyhow!(e))?;
        let ciphertext = age::encrypt_and_armor(&recipient, secret.as_bytes())?;

        // Send to client for decryption
        let auth_secret = Auth {
            pub_key: auth.pub_key,
            ciphertext,
            plaintext: "".to_string(),
        };
        self.socket
            .send(ServerMsg::AuthSecret(auth_secret).to_ws_msg())
            .await?;
        Ok(())
    }

    /// Handle the client sending back the decrypted auth secret
    async fn handle_auth_plaintext(&mut self, auth: Auth) -> Result<()> {
        info!(
            "✍️ Client {} attempting to authenticate as {}, sent back plaintext",
            self.peer_addr, auth.pub_key
        );

        // User cannot be authenticated twice at the same time
        {
            let user_conns_read = self.user_conns.read().await;
            if user_conns_read.contains_key(&auth.pub_key) {
                error!(
                    "✍️ Client {} failed authenticating as {}, user is already authenticated",
                    self.peer_addr, auth.pub_key
                );
                self.socket
                    .send(ServerMsg::AuthDenied(auth).to_ws_msg())
                    .await?;
                return Ok(());
            }
        }

        // Check decryption
        let auth_secret = self
            .auth_secret
            .clone()
            .ok_or(anyhow!("No auth secret set, cannot check"))?;
        if auth_secret != auth.plaintext {
            error!(
                "✍️ Client {} failed authenticating as {}, incorrect plaintext",
                self.peer_addr, auth.pub_key
            );
            self.socket
                .send(ServerMsg::AuthDenied(auth).to_ws_msg())
                .await?;
            return Ok(());
        }

        // Add username and note_tx to user_conns
        let mut user_conns_write = self.user_conns.write().await;
        // TODO: this is not secure, client could authenticate as different pub key
        // than the message was encrypted for
        user_conns_write.insert(auth.pub_key.clone(), self.note_tx.clone());
        info!(
            "✍️ Client {} successfully authenticated as {}",
            self.peer_addr, auth.pub_key
        );
        self.pub_key = Some(auth.pub_key.clone());
        self.socket
            .send(ServerMsg::AuthGranted(auth).to_ws_msg())
            .await?;
        Ok(())
    }

    /// Handle the client sending a note
    async fn handle_send_note(&mut self, note: Note) -> Result<()> {
        info!(
            "✉️ Client {} sent note from {} to {}",
            self.peer_addr, note.from, note.to
        );
        // TODO: what if from address does not match?
        // Echo back the note so that it will be in the history
        self.socket
            .send(ServerMsg::RecNote(note.clone()).to_ws_msg())
            .await?;

        // Relay note to connection of recipient address
        let user_conns_read = self.user_conns.read().await;
        match user_conns_read.get(&note.to) {
            Some(recipient_tx) => {
                recipient_tx.send(note).await?;
            }
            None => {
                error!(
                    "✉️ Client {} sent note from {} to unauthenticated user {}",
                    self.peer_addr, note.from, note.to
                );
                // TODO: send back error message?
            }
        }

        Ok(())
    }
}
