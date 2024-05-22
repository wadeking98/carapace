use std::{collections::HashMap, net::IpAddr};

use async_std::net::TcpStream;
use rsa::RsaPublicKey;

use crate::shared::rpc::Request;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PendingNotification {
    recipients: Vec<String>,
    notification: Request,
}
impl PendingNotification {
    pub fn new(recipients: Vec<String>, notification: Request) -> Self {
        PendingNotification {
            recipients,
            notification,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerConfig{
    pub ip: IpAddr,
    pub port: u16
}

pub struct Server {
    pub connections: HashMap<String, TcpStream>,
    pub ip: IpAddr,
    pub port: u16,
}
impl Server {
    pub fn new(
        connections: HashMap<String, TcpStream>,
        ip: IpAddr,
        port: u16,
    ) -> Self {
        Server {
            connections,
            ip,
            port,
        }
    }
}
