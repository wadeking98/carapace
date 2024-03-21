use std::error::Error;
use std::time::Duration;

use async_std::net::TcpListener;
use async_std::{prelude::*, task};
use rsa::pkcs1::pem::Base64Encoder;
use rsa::pkcs1v15::Signature;
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::models::EncryptionConfiguration;
use crate::shared::rpc_models::{
    self, ClientEncryptionPackage, RespondClientChallenge, RespondServerChallenge,
};
use crate::shared::{
    pki,
    rpc::{self, Handler, Request, Response, RpcError, RpcErrorCode},
    ski,
};

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
#[derive(Clone)]
pub struct ServerHandler {
    encryption: Option<EncryptionConfiguration>,
    server_private_key: RsaPrivateKey,
    pending_challenge: Option<String>,
}
impl ServerHandler {
    pub fn new(server_private_key: RsaPrivateKey) -> Self {
        ServerHandler {
            encryption: None,
            server_private_key,
            pending_challenge: None,
        }
    }

    fn handle_get_encryption_package(&self, request: Request) -> Result<Response, Box<dyn Error>> {
        let method = request.method.as_str();
        if method == rpc_models::REQUEST_ENCRYPTION_PACKAGE {
            if self.encryption.is_none() {
                return Err("Encryption not initialized".into());
            }

            if let Some(ref encryption) = self.encryption {
                let package = ClientEncryptionPackage::new(
                    encryption.nonce.clone(),
                    encryption.shared_key.clone(),
                );
                return Ok(Response::new(serde_json::json!(package), None, request.id));
            } else {
                return Err("Encryption not initialized".into());
            }
        } else {
            Err("Invalid method".into())
        }
    }

    fn handle_ping(&self, request: Request) -> Result<Response, Box<dyn Error>> {
        let method = request.method.as_str();
        if method == rpc_models::PING {
            Ok(Response::new(serde_json::json!("pong"), None, request.id))
        } else {
            Err("Invalid method".into())
        }
    }

    fn handle_encrypted_request(&mut self, request: Request) -> Result<Response, Box<dyn Error>> {
        let method = request.method.as_str();
        let req_id = request.id.clone();
        if method == rpc_models::ENCRYPTED_REQUEST {
            if self.encryption.is_none() {
                return Err("Encryption not initialized".into());
            }
            let enc_params: rpc_models::EncryptedRequestParams =
                serde_json::from_value(request.params)?;
            let data = enc_params.data;
            let enc_type = enc_params.enc_type;
            let request = match enc_type {
                rpc_models::EncryptionType::RsaPkcs1v15 => {
                    let data = pki::decrypt_message(&self.server_private_key, &data)?;
                    let request: Request = serde_json::from_slice(&data)?;
                    request
                }
                rpc_models::EncryptionType::AesGcm => {
                    let key = &self.encryption.as_ref().unwrap().shared_key;
                    let nonce = &self.encryption.as_ref().unwrap().nonce;
                    let data = ski::decrypt_gcm(&data, key, nonce)?;
                    let request: Request = serde_json::from_slice(&data)?;
                    request
                }
            };
            let error_handler = |e: Box<dyn Error>| {
                Response::new(
                    serde_json::json!(null),
                    Some(RpcError {
                        message: String::from(e.to_string()),
                        code: RpcErrorCode::InvalidRequest,
                    }),
                    req_id.clone(),
                )
            };
            let response = match request.method.as_str() {
                rpc_models::REQUEST_ENCRYPTION_PACKAGE => self
                    .handle_get_encryption_package(request)
                    .unwrap_or_else(error_handler),
                rpc_models::PING => self.handle_ping(request).unwrap_or_else(error_handler),
                _ => Response::new(
                    serde_json::json!(null),
                    Some(RpcError {
                        message: String::from("Invalid rpc method"),
                        code: RpcErrorCode::MethodNotFound,
                    }),
                    req_id.clone(),
                ),
            };

            let enc_response = match enc_type {
                rpc_models::EncryptionType::RsaPkcs1v15 => {
                    let data = serde_json::json!(&response);
                    let data = pki::encrypt_message(
                        &self.encryption.as_ref().unwrap().pub_key,
                        data.to_string().as_bytes(),
                    )?;
                    data
                }
                rpc_models::EncryptionType::AesGcm => {
                    let data = serde_json::json!(&response);
                    let key = &self.encryption.as_ref().unwrap().shared_key;
                    let nonce = &self.encryption.as_ref().unwrap().nonce;
                    let data = ski::encrypt_gcm(data.to_string().as_bytes(), key, nonce)?;
                    data
                }
            };

            Ok(Response::new(serde_json::json!(enc_response), None, req_id))
        } else {
            Err("Invalid method".into())
        }
    }
    fn handle_start_server_handshake(
        &mut self,
        request: Request,
    ) -> Result<Response, Box<dyn Error>> {
        let method = request.method.as_str();
        if method == rpc_models::START_SERVER_HANDSHAKE {
            let challenge = Uuid::new_v4().to_string();
            self.pending_challenge = Some(challenge.clone());
            Ok(Response::new(
                serde_json::json!(challenge),
                None,
                request.id,
            ))
        } else {
            Err("Invalid method".into())
        }
    }
    fn handle_challenge_response(&mut self, request: Request) -> Result<Response, Box<dyn Error>> {
        let method = request.method.as_str();
        if method == rpc_models::CLIENT_CHALLENGE_RESPONSE {
            let response: RespondClientChallenge = serde_json::from_value(request.params)?;
            if self.pending_challenge.is_none() {
                return Err("No pending challenge".into());
            }
            let sig = Signature::try_from(response.signiture.as_slice())?;
            if !pki::verify_signature(
                &response.pub_key,
                self.pending_challenge.as_ref().unwrap().as_bytes(),
                &sig,
            ) {
                return Err("Invalid signature".into());
            }
            let nonce = ski::nonce();
            let shared_key = ski::gen_key();
            self.encryption = Some(EncryptionConfiguration {
                shared_key: shared_key.clone(),
                nonce: nonce.clone(),
                pub_key: response.pub_key.clone(),
            });
            let server_challenge = response.server_challenge.clone();
            let response = RespondServerChallenge {
                pub_key: self.server_private_key.to_public_key(),
                signiture: pki::sign_message(&self.server_private_key, server_challenge.as_bytes()),
            };
            Ok(Response::new(serde_json::json!(response), None, request.id))
        } else {
            Err("Invalid method".into())
        }
    }
}
impl Handler for ServerHandler {
    async fn handle(&mut self, request: Request) -> Response {
        let req_id = request.id.clone();
        let error_handler = |e: Box<dyn Error>| {
            Response::new(
                serde_json::json!(null),
                Some(RpcError {
                    message: String::from(e.to_string()),
                    code: RpcErrorCode::InvalidRequest,
                }),
                req_id.clone(),
            )
        };
        match request.method.as_str() {
            rpc_models::ENCRYPTED_REQUEST => self
                .handle_encrypted_request(request)
                .unwrap_or_else(error_handler),
            rpc_models::START_SERVER_HANDSHAKE => self
                .handle_start_server_handshake(request)
                .unwrap_or_else(error_handler),
            rpc_models::CLIENT_CHALLENGE_RESPONSE => self
                .handle_challenge_response(request)
                .unwrap_or_else(error_handler),
            _ => Response::new(
                serde_json::json!(null),
                Some(RpcError {
                    message: String::from("Invalid rpc method"),
                    code: RpcErrorCode::MethodNotFound,
                }),
                req_id,
            ),
        }
    }
}

pub struct Server {
    private_key: RsaPrivateKey,
    authorized_keys: Vec<RsaPublicKey>,
    config: ServerConfig,
    ip: String,
    port: u16,
}
impl Server {
    pub fn new(
        private_key: RsaPrivateKey,
        authorized_keys: Vec<RsaPublicKey>,
        ip: String,
        port: u16,
        config: Option<ServerConfig>,
    ) -> Self {
        Server {
            private_key,
            authorized_keys,
            ip,
            port,
            config: config.unwrap_or_default(),
        }
    }

    pub async fn start<H: Handler + Clone + Send + Sync + 'static>(
        &self,
        handler: H,
    ) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).await?;
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
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::server::tests::pki::{decrypt_message, encrypt_message};

    use super::*;
    use async_std::net::TcpStream;
    use async_std::{sync::RwLock, task};

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
                "127.0.0.1".to_string(),
                8888,
                None,
            );

            server.start(handler_write).await.unwrap();
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
        let handler = ServerHandler::new(server_private_key.clone());
        let server = Server::new(
            server_private_key,
            Vec::new(),
            "127.0.0.1".to_string(),
            8889,
            None,
        );
        task::spawn(async move {
            server.start(handler).await.unwrap();
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
