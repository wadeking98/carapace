use crate::shared::ski;
use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;
use std::error::Error;
use uuid::Uuid;

pub struct TransactionDatabase {
    pub user_db: EntryDb,
    pub message_db: EntryDb,
    pub server_db: EntryDb,
    pub chat_db: EntryDb,
}
impl TransactionDatabase {
    pub fn new(key: &[u8]) -> Result<Self, Box<dyn Error>> {
        let project_dirs = ProjectDirs::from("com", "carapace", "client")
            .ok_or("Could not find project directories")
            .unwrap();
        let base = project_dirs.config_dir();
        let user_db = EntryDb::new(key, sled::open(base.join("users.db"))?);
        let message_db = EntryDb::new(key, sled::open(base.join("messages.db"))?);
        let server_db = EntryDb::new(key, sled::open(base.join("server.db"))?);
        let chat_db = EntryDb::new(key, sled::open(base.join("chats.db"))?);
        Ok(Self {
            user_db,
            message_db,
            server_db,
            chat_db,
        })
    }
}

pub struct EntryDb {
    db: Db,
    key: Vec<u8>,
}
impl EntryDb {
    pub fn new(key: &[u8], db: Db) -> Self {
        Self {
            db,
            key: Vec::from(key),
        }
    }
    pub fn get_entry<I: Serialize + DeserializeOwned>(
        &self,
        id: &str,
    ) -> Result<I, Box<dyn Error>> {
        let entry = self.db.get(id)?;
        let entry = entry.ok_or("Id not found")?;
        let entry: Entry = serde_json::from_str(std::str::from_utf8(&entry)?)?;
        let nonce = entry.nonce;
        let value = ski::decrypt_gcm(&entry.value, &self.key, &nonce)?;
        let value: I = serde_json::from_str(std::str::from_utf8(&value)?)?;
        Ok(value)
    }

    pub fn get_all_entries<I: Serialize + DeserializeOwned>(
        &self,
    ) -> Result<Vec<(String, I)>, Box<dyn Error>> {
        let mut entries = vec![];
        for entry in self.db.iter() {
            let entry = entry?;
            let id = entry.0;
            let entry: Entry = serde_json::from_str(std::str::from_utf8(&entry.1)?)?;
            let nonce = entry.nonce;
            let value = ski::decrypt_gcm(&entry.value, &self.key, &nonce)?;
            let value: I = serde_json::from_str(std::str::from_utf8(&value)?)?;
            entries.push((String::from_utf8(id.to_vec())?, value));
        }
        Ok(entries)
    }

    pub fn update_entry<I: Serialize + DeserializeOwned>(
        &self,
        id: &str,
        entry: I,
    ) -> Result<(), Box<dyn Error>> {
        let serialized_entry = serde_json::to_string(&entry)?;
        let nonce = ski::nonce();
        let serialized_entry = ski::encrypt_gcm(serialized_entry.as_bytes(), &self.key, &nonce)?;
        let serialized_entry = Entry::new(nonce, serialized_entry);
        let serialized_entry = serde_json::to_string(&serialized_entry)?;
        self.db.insert(id, serialized_entry.as_str())?;
        Ok(())
    }

    pub fn save_entry<I: Serialize + DeserializeOwned>(
        &self,
        entry: I,
    ) -> Result<String, Box<dyn Error>> {
        let nonce = ski::nonce();
        let id = Uuid::new_v4().to_string();
        let serialized_entry = serde_json::to_string(&entry)?;
        let serialized_entry = ski::encrypt_gcm(serialized_entry.as_bytes(), &self.key, &nonce)?;
        let serialized_entry = Entry::new(nonce, serialized_entry);
        let serialized_entry = serde_json::to_string(&serialized_entry)?;
        self.db.insert(id.clone(), serialized_entry.as_str())?;
        Ok(id)
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), Box<dyn Error>> {
        self.db.remove(id)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Entry {
    nonce: Vec<u8>,
    value: Vec<u8>,
}
impl Entry {
    fn new(nonce: Vec<u8>, value: Vec<u8>) -> Self {
        Self { nonce, value }
    }
}

#[cfg(test)]
mod tests {

    use std::net::IpAddr;

    use super::*;

    use crate::client::models::ServerModel;

    #[test]
    fn test_save_entry() {

        // test save and delete entry
        let key = b"an example very very secret key";
        let db = TransactionDatabase::new(key).unwrap();
        db.server_db.db.clear().unwrap();
        let shared_key = ski::gen_key();
        let nonce = ski::nonce();
        let srv = ServerModel::new(
            String::from("server_id"),
            vec![],
            vec![],
            IpAddr::V4([127, 0, 0, 1].into()),
            8080,
        );
        let id = db.server_db.save_entry(srv).unwrap();
        let srv = db.server_db.get_entry::<ServerModel>(&id);
        assert!(srv.is_ok());
        db.server_db.delete_entry(&id).unwrap();
        let srv = db.server_db.get_entry::<ServerModel>(&id);
        assert!(srv.is_err());

        // test update entry
        let srv = ServerModel::new(
            String::from("server_id"),
            vec![],
            vec![],
            IpAddr::V4([127, 0, 0, 1].into()),
            8080,
        );
        let id = db.server_db.save_entry(srv).unwrap();
        let srv = ServerModel::new(
            String::from("server_id_new"),
            vec![],
            vec![],
            IpAddr::V4([127, 0, 0, 1].into()),
            8080,
        );
        db.server_db.update_entry(id.as_str(), srv).unwrap();
        let srv = db.server_db.get_entry::<ServerModel>(&id).unwrap();
        assert_eq!(srv.server_name, "server_id_new");
        db.server_db.delete_entry(&id).unwrap();

        // test get all entries
        let srv = ServerModel::new(
            String::from("server_id1"),
            vec![],
            vec![],
            IpAddr::V4([127, 0, 0, 1].into()),
            8080,
        );
        let id1 = db.server_db.save_entry(srv).unwrap();
        let srv = ServerModel::new(
            String::from("server_id2"),
            vec![],
            vec![],
            IpAddr::V4([127, 0, 0, 1].into()),
            8080,
        );
        let id2 = db.server_db.save_entry(srv).unwrap();
        let entries = db.server_db.get_all_entries::<ServerModel>().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().find(|(id, _)| id == &id1).is_some());
        assert!(entries.iter().find(|(id, _)| id == &id2).is_some());
    }
}
