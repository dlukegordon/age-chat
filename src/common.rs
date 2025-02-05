use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerWsMsg {
    RecNewMsg(RecNewMsg),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientWsMsg {
    SendNewMsg(SendNewMsg),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecNewMsg {
    content: String,
    timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendNewMsg {
    content: String,
    timestamp: DateTime<Utc>,
}
