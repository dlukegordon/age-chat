use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use tokio_tungstenite::tungstenite::Message;

/// WS Messages that the server sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    RecNewNote(RecNewNote),
}

/// WS Messages that the client sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    SendNewNote(SendNewNote),
}

/// Signal the client they have received a new chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecNewNote {
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Signal the server to send a new chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendNewNote {
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

    pub fn rec_new_note(content: String) -> Self {
        Self::RecNewNote(RecNewNote::new(content))
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

    pub fn send_new_note(content: String) -> Self {
        Self::SendNewNote(SendNewNote::new(content))
    }
}

impl RecNewNote {
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
        }
    }
}

impl SendNewNote {
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
        }
    }
}
