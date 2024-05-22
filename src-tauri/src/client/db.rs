use crate::shared::{db::EntryDb, ski};
use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;
use std::error::Error;
use uuid::Uuid;

pub struct ClientDatabase {
    pub known_user_db: EntryDb,
    pub message_db: EntryDb,
    pub server_db: EntryDb,
    pub chat_db: EntryDb,
}
impl ClientDatabase {
    pub fn new(key: &[u8]) -> Result<Self, Box<dyn Error>> {
        let project_dirs = ProjectDirs::from("com", "carapace", "client")
            .ok_or("Could not find project directories")
            .unwrap();
        let base = project_dirs.config_dir();
        let known_user_db = EntryDb::new(key, sled::open(base.join("known_users.db"))?);
        let message_db = EntryDb::new(key, sled::open(base.join("messages.db"))?);
        let server_db = EntryDb::new(key, sled::open(base.join("server.db"))?);
        let chat_db = EntryDb::new(key, sled::open(base.join("chats.db"))?);
        Ok(Self {
            known_user_db,
            message_db,
            server_db,
            chat_db,
        })
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
        let db = ClientDatabase::new(key).unwrap();
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
