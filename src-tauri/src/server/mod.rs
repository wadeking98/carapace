use std::error::Error;
use std::time::Duration;

use async_std::net::TcpListener;
use async_std::{prelude::*, task};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};


use crate::shared::rpc::{self, Handler};
pub mod handler;
pub mod models;

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    open_registration: bool,
    timeout: Duration,
}
impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            open_registration: false,
            timeout: Duration::from_secs(10),
        }
    }
}

pub struct Server {
    pub private_key: RsaPrivateKey,
    authorized_keys: Vec<RsaPublicKey>,
    config: ServerConfig,
}
impl Server {
    pub fn new(
        private_key: RsaPrivateKey,
        authorized_keys: Vec<RsaPublicKey>,
        config: Option<ServerConfig>,
    ) -> Self {
        Server {
            private_key,
            authorized_keys,
            config: config.unwrap_or_default(),
        }
    }
}
pub async fn start_server<H: Handler + Clone + Send + Sync + 'static>(
    handler: H,
    ip: String,
    port: u16,
) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("{}:{}", ip, port)).await?;
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let mut stream = stream?;
        let mut handler = handler.clone();
        task::spawn(async move {
            if let Err(e) = rpc::listen(&mut stream, &mut handler).await {
                eprintln!("Error: {}", e);
            }
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::shared::pki::{decrypt_message, encrypt_message};
    use crate::shared::rpc_models::{ClientEncryptionPackage, RespondClientChallenge, RespondServerChallenge};
    use crate::shared::{pki, rpc_models};
    use crate::shared::rpc::{Request, Response};

    use self::handler::ServerHandler;

    use super::*;
    use async_std::net::TcpStream;
    use async_std::{sync::RwLock, task};
    use rsa::pkcs1v15::Signature;

    #[test]
    fn test_server() {
        #[derive(Clone)]
        struct TestHandler {
            test: Arc<RwLock<bool>>,
        }
        impl TestHandler {
            pub fn new() -> Self {
                TestHandler {
                    test: Arc::new(RwLock::new(false)),
                }
            }
        }
        impl Handler for TestHandler {
            async fn handle(&mut self, request: Request) -> Response {
                {
                    let mut gaurd = self.test.write().await;
                    *gaurd = true;
                }
                Response::new(serde_json::json!("test"), None, request.id)
            }
        }
        let handler = TestHandler::new();
        let handler_write = handler.clone();
        let handler_read = handler.clone();
        let server_private_key = pki::gen_key().unwrap();
        task::spawn(async {
            let server = Server::new(
                server_private_key,
                Vec::new(),
                None,
            );

            start_server(handler_write, String::from("127.0.0.1"), 8888).await.unwrap();
        });
        task::block_on(async {
            task::sleep(Duration::from_secs(1)).await;
            let mut stream = TcpStream::connect("127.0.0.1:8888").await.unwrap();
            assert!(stream.peer_addr().is_ok());
            let request = Request::new("test".to_string(), serde_json::json!("test"));
            let response = request.send(&mut stream, None).await.unwrap();
            assert_eq!(response.result, serde_json::json!("test"));
            assert!(handler_read.test.read().await.clone());
        });
    }

    #[test]
    fn test_default_handler() {
        let server_private_key = pki::gen_key().unwrap();
        let server = Server::new(
            server_private_key,
            Vec::new(),
            None,
        );
        let handler = ServerHandler::new(Arc::new(RwLock::new(server)));
        task::spawn(async move {
            start_server(handler, String::from("127.0.0.1"), 8889).await.unwrap();
        });
        task::block_on(async {
            task::sleep(Duration::from_secs(1)).await;
            let mut stream = TcpStream::connect("127.0.0.1:8889").await.unwrap();
            assert!(stream.peer_addr().is_ok());
            let request = Request::new(
                rpc_models::START_SERVER_HANDSHAKE.to_string(),
                serde_json::json!(null),
            );
            let response = request.send(&mut stream, None).await.unwrap();
            let challenge: String = serde_json::from_value(response.result).unwrap();
            assert!(challenge.len() == 36);
            let private_key = pki::gen_key().unwrap();
            let challenge = challenge.as_bytes();
            let sig = pki::sign_message(&private_key, challenge);
            let server_challenge = uuid::Uuid::new_v4().to_string();
            let response = RespondClientChallenge {
                pub_key: private_key.to_public_key(),
                signiture: sig,
                server_challenge: server_challenge.clone(),
            };
            let request = Request::new(
                rpc_models::CLIENT_CHALLENGE_RESPONSE.to_string(),
                serde_json::json!(response),
            );
            let response = request.send(&mut stream, None).await.unwrap();
            let response: RespondServerChallenge = serde_json::from_value(response.result).unwrap();
            let sig = Signature::try_from(response.signiture.as_slice()).unwrap();
            let server_pub_key = response.pub_key;
            assert!(pki::verify_signature(
                &server_pub_key,
                server_challenge.as_bytes(),
                &sig
            ));

            let request = Request::new(
                rpc_models::REQUEST_ENCRYPTION_PACKAGE.to_string(),
                serde_json::json!(null),
            );
            let req_id = request.id.clone();
            let encrypted_request = encrypt_message(
                &server_pub_key,
                serde_json::json!(request).to_string().as_bytes(),
            )
            .unwrap();
            let request_params = rpc_models::EncryptedRequestParams {
                enc_type: rpc_models::EncryptionType::RsaPkcs1v15,
                data: encrypted_request,
            };
            let request = Request::new_with_id(
                rpc_models::ENCRYPTED_REQUEST.to_string(),
                serde_json::json!(request_params),
                req_id,
            );
            let response = request.send(&mut stream, None).await.unwrap();
            let ct: Vec<u8> = serde_json::from_value(response.result).unwrap();
            let response = decrypt_message(&private_key, &ct).unwrap();
            let response: Response = serde_json::from_slice(&response).unwrap();
            let package: ClientEncryptionPackage = serde_json::from_value(response.result).unwrap();
            println!("{:?}", package.shared_key());
        });
    }
}
