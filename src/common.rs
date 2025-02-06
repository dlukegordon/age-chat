use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// WS Messages that the server sends
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerWsMsg {
    RecNewNote(RecNewNote),
}

/// WS Messages that the client sends
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientWsMsg {
    SendNewNote(SendNewNote),
}

/// Signal the client they have received a new chat message
#[derive(Serialize, Deserialize, Debug)]
pub struct RecNewNote {
    content: String,
    timestamp: DateTime<Utc>,
}

/// Signal the server to send a new chat message
#[derive(Serialize, Deserialize, Debug)]
pub struct SendNewNote {
    content: String,
    timestamp: DateTime<Utc>,
}

impl FromStr for ServerWsMsg {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl FromStr for ClientWsMsg {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl fmt::Display for ServerWsMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

impl fmt::Display for ClientWsMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}
