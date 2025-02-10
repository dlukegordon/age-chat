use age::{
    armor::{ArmoredWriter, Format},
    x25519::{Identity, Recipient},
    Encryptor,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, io::Write, str::FromStr};
use tokio_tungstenite::tungstenite::Message;

pub const CHANNEL_BUFFER_SIZE: usize = 1000;

/// WS Messages that the server sends
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    /// Send the client a secret to decrypt to authenticate
    AuthSecret(Auth),
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
    /// Return the decrypted secret to authenticate
    AuthPlaintext(Auth),
    /// Signal the server to send a new chat message
    SendNote(Note),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Auth {
    pub pub_key: String,
    pub ciphertext: String,
    pub plaintext: String,
}

/// A chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub from: String,
    pub to: String,
    pub encrypted_content: String,
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
    pub fn new(pub_key: String) -> Self {
        Self {
            pub_key,
            ciphertext: "".into(),
            plaintext: "".into(),
        }
    }
}

impl Note {
    pub fn encrypt_new(from: &Recipient, to: &Recipient, content: String) -> Result<Self> {
        // Encrypt to from and to pubkeys
        let recipients: Vec<&dyn age::Recipient> = vec![from, to];
        let encryptor = Encryptor::with_recipients(recipients.into_iter())?;
        let mut encrypted_content = vec![];
        let mut writer = encryptor.wrap_output(ArmoredWriter::wrap_output(
            &mut encrypted_content,
            Format::AsciiArmor,
        )?)?;
        writer.write_all(&content.into_bytes())?;
        writer.finish()?.finish()?;
        let encrypted_content = String::from_utf8(encrypted_content)?;

        Ok(Self {
            from: from.to_string(),
            to: to.to_string(),
            encrypted_content,
            timestamp: Utc::now(),
        })
    }

    pub fn decrypt_content(&self, priv_key: &Identity) -> Result<String> {
        Ok(String::from_utf8(age::decrypt(
            priv_key,
            self.encrypted_content.as_bytes(),
        )?)?)
    }
}
