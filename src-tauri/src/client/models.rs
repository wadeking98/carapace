use std::{net::IpAddr, time::SystemTime};

use crate::shared::models::EncryptionConfiguration;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    username: String,
    pub_key: String,
}
impl User {
    pub fn new(username: String, pub_key: String) -> Self {
        User { username, pub_key }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Message {
    server_id: String,
    sender_id: Option<String>,
    chat_id: String,
    message: String,
    timestamp: SystemTime,
}
impl Message {
    pub fn new(
        server_id: String,
        sender_id: Option<String>,
        chat_id: String,
        message: String,
    ) -> Self {
        Message {
            server_id,
            sender_id,
            chat_id,
            message,
            timestamp: SystemTime::now(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Chat {
    user_id: String,
    name: String,
    shared_key: Vec<u8>,
    nonce: Vec<u8>,
    message_ids: Vec<String>,
    last_message_id: String,
}
impl Chat {
    pub fn new(user_id: String, name: String, shared_key: Vec<u8>, nonce: Vec<u8>) -> Self {
        Chat {
            user_id,
            name,
            shared_key,
            nonce,
            message_ids: Vec::new(),
            last_message_id: String::new(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ServerModel {
    pub server_name: String,
    pub encryption: Option<EncryptionConfiguration>,
    user_ids: Vec<String>,
    chat_ids: Vec<String>,
    pub ip: IpAddr,
    pub port: u16,
}
impl ServerModel {
    pub fn new(
        server_name: String,
        user_ids: Vec<String>,
        chat_ids: Vec<String>,
        ip: IpAddr,
        port: u16,
    ) -> Self {
        ServerModel {
            server_name,
            encryption: None,
            user_ids,
            chat_ids,
            ip,
            port,
        }
    }
    pub fn add_encryption(&mut self, encryption: EncryptionConfiguration) {
        self.encryption = Some(encryption);
    }
}
