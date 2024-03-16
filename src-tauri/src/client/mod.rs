use std::error::Error;

use async_std::net::TcpStream;
use rsa::{pkcs1v15::Signature, RsaPrivateKey};

use crate::shared::{
    models::EncryptionConfiguration,
    pki::{
        decrypt_message, encrypt_message, gen_key, key_exists, read_key_from_file, sign_message,
        verify_signature, write_key_to_file,
    },
    rpc::{Request, Response},
    rpc_models::{self, ClientEncryptionPackage, RespondClientChallenge, RespondServerChallenge},
};

use self::db::TransactionDatabase;

mod db;
mod models;

struct Client {
    private_key: RsaPrivateKey,
    db: TransactionDatabase,
    server_conn: Option<TcpStream>,
}
impl Client {
    pub fn new(pass_key: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        if !key_exists("client") {
            let key = gen_key()?;
            write_key_to_file(&key, "client", &pass_key)?;
        }
        let private_key = read_key_from_file("client", &pass_key)?;
        let db = TransactionDatabase::new(&pass_key)?;
        Ok(Client {
            private_key,
            db,
            server_conn: None,
        })
    }

    pub async fn server_connect(&mut self, server_id: &str) -> Result<(), Box<dyn Error>> {
        let mut server = self.db.server_db.get_entry::<models::Server>(server_id)?;
        let mut stream = TcpStream::connect((server.ip, server.port)).await?;
        let request = Request::new(
            rpc_models::START_SERVER_HANDSHAKE.to_string(),
            serde_json::json!(null),
        );
        let response = request.send(&mut stream, None).await?;
        let challenge: String = serde_json::from_value(response.result)?;
        let challenge = challenge.as_bytes();
        let sig = sign_message(&self.private_key, challenge);

        let server_challenge = uuid::Uuid::new_v4().to_string();

        let response = RespondClientChallenge {
            pub_key: self.private_key.to_public_key(),
            signiture: sig,
            server_challenge: server_challenge.clone(),
        };

        let request = Request::new(
            rpc_models::CLIENT_CHALLENGE_RESPONSE.to_string(),
            serde_json::json!(response),
        );
        let response = request.send(&mut stream, None).await?;
        let server_challenge_response: RespondServerChallenge =
            serde_json::from_value(response.result)?;
        let sig = Signature::try_from(server_challenge_response.signiture.as_slice())?;
        let server_pub_key = server_challenge_response.pub_key;

        // Verify the server's signature
        if !verify_signature(&server_pub_key, server_challenge.as_bytes(), &sig) {
            Err("Server verification failed")?;
        }

        // Get the shared key for faster encryption
        let request = Request::new(
            rpc_models::REQUEST_ENCRYPTION_PACKAGE.to_string(),
            serde_json::json!(null),
        );
        let req_id = request.id.clone();
        let encrypted_request = encrypt_message(
            &server_pub_key,
            serde_json::json!(request).to_string().as_bytes(),
        )?;
        let request_params = rpc_models::EncryptedRequestParams {
            enc_type: rpc_models::EncryptionType::RsaPkcs1v15,
            data: encrypted_request,
        };
        let request = Request::new_with_id(
            rpc_models::ENCRYPTED_REQUEST.to_string(),
            serde_json::json!(request_params),
            req_id,
        );

        let response = request.send(&mut stream, None).await?;
        let ct: Vec<u8> = serde_json::from_value(response.result)?;
        let response = decrypt_message(&self.private_key, &ct)?;
        let response: Response = serde_json::from_slice(&response)?;
        let package: ClientEncryptionPackage = serde_json::from_value(response.result)?;
        server.add_encryption(EncryptionConfiguration::new(
            package.shared_key(),
            package.nonce(),
            server_pub_key,
        ));
        self.db.server_db.update_entry(server_id, server)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use async_std::task;

    use crate::{
        server::{DefaultHandler, Server},
        shared::pki::delete_key_file,
    };

    use crate::client::models::Server as ServerModel;

    use super::*;

    #[test]
    fn test_client() {
        let client = Client::new(b"example key1".to_vec()).unwrap();
        let server_private_key = gen_key().unwrap();
        let handler = DefaultHandler::new(server_private_key.clone());
        let server_model = ServerModel::new(
            "test_server".to_string(),
            vec![],
            vec![],
            IpAddr::V4([127, 0, 0, 1].into()),
            8890,
        );

        let server_id = client
            .db
            .server_db
            .save_entry(server_model)
            .expect("Failed to save server");

        let server = Server::new(
            server_private_key,
            Vec::new(),
            "127.0.0.1".to_string(),
            8890,
            None,
        );
        task::spawn(async move {
            server.start(handler).await.unwrap();
        });
        task::block_on(async {
            let mut client = client;
            client
                .server_connect(&server_id)
                .await
                .expect("Failed to connect to server");
            let updated_server = client
                .db
                .server_db
                .get_entry::<ServerModel>(&server_id)
                .expect("Failed to get server");
            assert!(updated_server.encryption.is_some());
        });
        delete_key_file("client").unwrap();
    }
}
