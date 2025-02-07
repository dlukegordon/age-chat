use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use tokio_tungstenite::tungstenite::Message;

/// WS Messages that the server sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    RecNote(RecNote),
}

/// WS Messages that the client sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    SendNote(SendNote),
}

/// Signal the client they have received a new chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecNote {
    pub note: Note,
}

/// Signal the server to send a new chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendNote {
    pub note: Note,
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
