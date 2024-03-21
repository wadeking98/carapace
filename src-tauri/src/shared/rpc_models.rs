use rsa::{pkcs1v15::Signature, RsaPublicKey};
use base64::{prelude::BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum EncryptionType {
    AesGcm,
    RsaPkcs1v15,
}

#[derive(Serialize, Deserialize)]
pub struct RespondClientChallenge {
    pub pub_key: RsaPublicKey,
    pub signiture: Vec<u8>,
    pub server_challenge: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RespondServerChallenge{
    pub pub_key: RsaPublicKey,
    pub signiture: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientEncryptionPackage {
    nonce: String,
    shared_key: String,
}
impl ClientEncryptionPackage {
    pub fn new(nonce: Vec<u8>, shared_key: Vec<u8>) -> Self {
        let nonce = BASE64_STANDARD.encode(nonce);
        let shared_key = BASE64_STANDARD.encode(shared_key);
        ClientEncryptionPackage { nonce, shared_key }
    }
    pub fn nonce(&self) -> Vec<u8> {
        BASE64_STANDARD.decode(self.nonce.clone()).unwrap()
    }
    pub fn shared_key(&self) -> Vec<u8> {
        BASE64_STANDARD.decode(self.shared_key.clone()).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct EncryptedRequestParams{
    pub enc_type: EncryptionType,
    pub data: Vec<u8>,
}

pub const START_SERVER_HANDSHAKE: &str = "start_server_handshake";
pub const CLIENT_CHALLENGE_RESPONSE: &str = "client_challenge_response";


pub const ENCRYPTED_REQUEST: &str = "encrypted_request";
pub const REQUEST_ENCRYPTION_PACKAGE: &str = "request_encryption_package";

pub const PING: &str = "ping";
