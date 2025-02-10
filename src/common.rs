use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use tokio_tungstenite::tungstenite::Message;

pub const CHANNEL_BUFFER_SIZE: usize = 1000;

/// WS Messages that the server sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    /// Signal the client that they have successfully authenticated
    AuthGranted(Auth),
    /// Signal the client that they have failed authentication
    AuthDenied(Auth),
    /// Signal the client they have received a new chat message
    RecNote(Note),
}

/// WS Messages that the client sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    /// Request the server to authenticate as a user
    AuthReq(Auth),
    /// Signal the server to send a new chat message
    SendNote(Note),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Auth {
    pub username: String,
}

/// A chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl FromStr for ServerMsg {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl fmt::Display for ServerMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

impl ServerMsg {
    pub fn to_ws_msg(&self) -> Message {
        Message::text(self.to_string())
    }
}

impl FromStr for ClientMsg {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl fmt::Display for ClientMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

impl ClientMsg {
    pub fn to_ws_msg(&self) -> Message {
        Message::text(self.to_string())
    }
}

impl Auth {
    pub fn new(username: String) -> Self {
        Self { username }
    }
}

impl Note {
    pub fn new(from: String, to: String, content: String) -> Self {
        Self {
            from,
            to,
            content,
            timestamp: Utc::now(),
        }
    }
}
