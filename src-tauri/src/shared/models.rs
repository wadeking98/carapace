use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptionConfiguration {
    pub shared_key: Vec<u8>,
    pub nonce: Vec<u8>,
}
impl EncryptionConfiguration {
    pub fn new(shared_key: Vec<u8>, nonce: Vec<u8>,) -> Self {
        EncryptionConfiguration {
            shared_key,
            nonce,
        }
    }
}
