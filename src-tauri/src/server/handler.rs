use std::error::Error;
use std::sync::Arc;

use async_std::sync::RwLock;
use rsa::pkcs1v15::Signature;
use rsa::RsaPublicKey;
use uuid::Uuid;

use crate::shared::{pki, ski};
use crate::shared::rpc::{Handler, Request, Response, RpcError, RpcErrorCode};
use crate::shared::models::EncryptionConfiguration;
use crate::shared::rpc_models::{self, ClientEncryptionPackage, RespondClientChallenge, RespondServerChallenge};

use super::Server;


#[derive(Clone)]
pub struct ServerHandler {
    server: Arc<RwLock<Server>>,
    encryption: Option<EncryptionConfiguration>,
    client_pub_key: Option<RsaPublicKey>,
    pending_challenge: Option<String>,
}
impl ServerHandler {
    pub fn new(server: Arc<RwLock<Server>>) -> Self {
        ServerHandler {
            server,
            encryption: None,
            client_pub_key: None,
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

    fn handle_forwarded_msg(&self, request: Request) -> Result<Response, Box<dyn Error>>{
        let method = request.method.as_str();
        if method == rpc_models::FORWARDED_MSG {
            let msg: rpc_models::ForwardedMessageParams = serde_json::from_value(request.params)?;
            Ok(Response::new(serde_json::json!(msg), None, request.id))
        } else {
            Err("Invalid method".into())
        }
    }

    async fn handle_encrypted_request(&mut self, request: Request) -> Result<Response, Box<dyn Error>> {
        let method = request.method.as_str();
        let req_id = request.id.clone();
        if method == rpc_models::ENCRYPTED_REQUEST {
            if self.encryption.is_none() || self.client_pub_key.is_none() {
                return Err("Encryption not initialized".into());
            }
            let enc_params: rpc_models::EncryptedRequestParams =
                serde_json::from_value(request.params)?;
            let data = enc_params.data;
            let enc_type = enc_params.enc_type;
            let request = match enc_type {
                rpc_models::EncryptionType::RsaPkcs1v15 => {
                    let data = pki::decrypt_message(&self.server.read().await.private_key, &data)?;
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
                        &self.client_pub_key.as_ref().unwrap(),
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
    async fn handle_challenge_response(&mut self, request: Request) -> Result<Response, Box<dyn Error>> {
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
            });
            self.client_pub_key = Some(response.pub_key.clone());
            let server_challenge = response.server_challenge.clone();
            let response = RespondServerChallenge {
                pub_key: self.server.read().await.private_key.to_public_key(),
                signiture: pki::sign_message(&self.server.read().await.private_key, server_challenge.as_bytes()),
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
                .await.unwrap_or_else(error_handler),
            rpc_models::START_SERVER_HANDSHAKE => self
                .handle_start_server_handshake(request)
                .unwrap_or_else(error_handler),
            rpc_models::CLIENT_CHALLENGE_RESPONSE => self
                .handle_challenge_response(request)
                .await.unwrap_or_else(error_handler),
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