use std::{error::Error, fs};

use base64::{prelude::BASE64_STANDARD, Engine};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs1v15::{Signature, SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, LineEnding};
use rsa::sha2::{Digest, Sha256};
use rsa::signature::{Keypair, RandomizedSigner, SignatureEncoding, Verifier};
use rsa::{Pkcs1v15Encrypt, Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};

use crate::shared::ski::{decrypt_gcm, encrypt_gcm, nonce};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use rand_core::OsRng;

pub fn gen_key() -> Result<RsaPrivateKey, Box<dyn Error>> {
    let mut csprng = OsRng {};
    let bits = 2048;
    let key = RsaPrivateKey::new(&mut csprng, bits)?;
    Ok(key)
}

#[cfg(target_os = "windows")]
pub fn get_line_ending() -> LineEnding {
    LineEnding::CRLF
}

#[cfg(not(target_os = "windows"))]
pub fn get_line_ending() -> LineEnding {
    LineEnding::LF
}

#[derive(Serialize, Deserialize)]
struct PEM {
    pem: Vec<u8>,
    nonce: Vec<u8>,
}

pub fn write_key_to_file(
    sk: &RsaPrivateKey,
    loc: &str,
    file_key: &[u8],
) -> Result<(), Box<dyn Error>> {
    let project_dirs =
        ProjectDirs::from("com", "carapace", loc).ok_or("Could not find project directories")?;
    let config_dir = project_dirs.config_dir();
    fs::create_dir_all(config_dir)?;
    let key_path = config_dir.join("private_key.pem");
    let pem = sk.to_pkcs8_pem(get_line_ending())?;
    let nonce = nonce();
    let pem_enc = encrypt_gcm(pem.as_bytes(), file_key, &nonce)?;
    let pem_struct = PEM {
        pem: pem_enc,
        nonce,
    };
    let pem_json = serde_json::to_string(&pem_struct)?;
    fs::write(key_path, pem_json)?;
    Ok(())
}

pub fn read_key_from_file(loc: &str, file_key: &[u8]) -> Result<RsaPrivateKey, Box<dyn Error>> {
    let project_dirs =
        ProjectDirs::from("com", "carapace", loc).ok_or("Could not find project directories")?;
    let key_path = project_dirs.config_dir().join("private_key.pem");
    let pem_json = fs::read_to_string(key_path)?;
    let pem_struct: PEM = serde_json::from_str(&pem_json)?;
    let pem = String::from_utf8(decrypt_gcm(&pem_struct.pem, file_key, &pem_struct.nonce)?)?;
    let sk = DecodePrivateKey::from_pkcs8_pem(pem.as_str())?;
    Ok(sk)
}

pub fn delete_key_file(loc: &str) -> Result<(), Box<dyn Error>> {
    let project_dirs =
        ProjectDirs::from("com", "carapace", loc).ok_or("Could not find project directories")?;
    let key_path = project_dirs.config_dir().join("private_key.pem");
    fs::remove_file(key_path)?;
    Ok(())
}

pub fn key_exists(loc: &str) -> bool {
    let project_dirs = ProjectDirs::from("com", "carapace", loc)
        .ok_or("Could not find project directories")
        .unwrap();
    let config_dir = project_dirs.config_dir();
    let path = config_dir.join("private_key.pem");
    path.exists()
}

pub fn sign_message(sk: &RsaPrivateKey, msg: &[u8]) -> Vec<u8> {
    let mut rng = OsRng {};
    let snk = SigningKey::<Sha256>::from(sk.clone());
    let sig = snk.sign_with_rng(&mut rng, msg);
    sig.to_bytes().to_vec()
}

pub fn verify_signature(pk: &RsaPublicKey, msg: &[u8], sig: &Signature) -> bool {
    let vk = VerifyingKey::<Sha256>::from(pk.clone());
    vk.verify(msg, sig).is_ok()
}

pub fn encrypt_message(pk: &RsaPublicKey, msg: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut rng = OsRng {};
    let ct = pk.encrypt(&mut rng, Pkcs1v15Encrypt, msg)?;
    Ok(ct)
}

pub fn decrypt_message(sk: &RsaPrivateKey, ct: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let pt = sk.decrypt(Pkcs1v15Encrypt, ct)?;
    Ok(pt)
}

pub fn pub_key_from_str(pk: &str) -> Result<RsaPublicKey, Box<dyn Error>> {
    let pk = RsaPublicKey::from_public_key_pem(pk)?;
    Ok(pk)
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_gen_key() {
        let sk = gen_key();
        assert!(sk.is_ok());
    }
    #[test]
    fn test_write_key_to_file() {
        let sk = gen_key().unwrap();
        let file_key = String::from("example key1");
        write_key_to_file(&sk, "client", file_key.as_bytes()).unwrap();
        assert_eq!(key_exists("client"), true);
        let sk_read = read_key_from_file("client", file_key.as_bytes()).unwrap();
        assert_eq!(sk, sk_read);
        delete_key_file("client").unwrap();
    }
    #[test]
    fn test_enc_dec_message() {
        let sk = gen_key().unwrap();
        let pk = RsaPublicKey::from(&sk);
        let msg = b"hello world";
        let ct = encrypt_message(&pk, msg).unwrap();
        let pt = decrypt_message(&sk, &ct).unwrap();
        assert_eq!(msg, pt.as_slice());
    }
    #[test]
    fn test_sign_message() {
        let sk = gen_key().unwrap();
        let msg = b"hello world";
        let sig = sign_message(&sk, msg);
        let pk = RsaPublicKey::from(&sk);
        let verified = verify_signature(&pk, msg, &Signature::try_from(sig.as_slice()).unwrap());
        assert!(verified);
        let sk2 = gen_key().unwrap();
        let pk = RsaPublicKey::from(&sk2);
        let verified2 = verify_signature(&pk, msg, &Signature::try_from(sig.as_slice()).unwrap());
        assert!(!verified2)
    }
}
