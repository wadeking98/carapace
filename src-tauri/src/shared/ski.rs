use std::error::Error;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm,
    Key, // Or `Aes128Gcm`
    Nonce,
};
use sha256::digest;

pub fn encrypt_gcm(pt: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let key = digest(key);
    let key = hex::decode(key)?;
    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(nonce);
    let ciphertext = cipher.encrypt(&nonce, pt).map_err(|e| e.to_string())?;
    Ok(ciphertext.to_vec())
}

pub fn decrypt_gcm(ct: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let key = digest(key);
    let key = hex::decode(key)?;
    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(nonce);
    let ciphertext = cipher.decrypt(&nonce, ct).map_err(|e| {println!("err {:?}", e); return e.to_string()})?;
    Ok(ciphertext.to_vec())
}

pub fn nonce() -> Vec<u8> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    nonce.to_vec()
}

pub fn gen_key() -> Vec<u8> {
    let key = Aes256Gcm::generate_key(&mut OsRng);
    key.to_vec()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_gcm() {
        let pt = b"Hello, world!";
        let key = gen_key();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = super::encrypt_gcm(pt, &key, &nonce).unwrap();
        let pt2 = super::decrypt_gcm(&ct, &key, &nonce).unwrap();
        assert_eq!(pt, pt2.as_slice());
    }
}
