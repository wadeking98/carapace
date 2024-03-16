use rsa::RsaPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptionConfiguration {
    pub shared_key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub pub_key: RsaPublicKey,
}
impl EncryptionConfiguration {
    pub fn new(shared_key: Vec<u8>, nonce: Vec<u8>, pub_key: RsaPublicKey) -> Self {
        EncryptionConfiguration {
            shared_key,
            nonce,
            pub_key,
        }
    }
}
