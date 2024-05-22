use std::error::Error;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;
use uuid::Uuid;

use super::ski;

pub struct EntryDb {
    pub db: Db,
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