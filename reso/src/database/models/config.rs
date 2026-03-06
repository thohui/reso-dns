use anyhow::Context;
use std::collections::HashMap;

use rusqlite::params;

use crate::database::DatabaseConnection;

pub struct ConfigSetting {
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

impl ConfigSetting {
    pub async fn get(db: &DatabaseConnection, key: &str) -> anyhow::Result<Option<String>> {
        let key = key.to_string();

        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare("SELECT value FROM config_settings WHERE key = ?1")?;
                let mut rows = stmt.query(params![key])?;
                let result = match rows.next()? {
                    Some(row) => Some(row.get::<_, String>(0)?),
                    None => None,
                };
                Ok::<_, rusqlite::Error>(result)
            })
            .await
            .context("failed to get config setting")?)
    }

    pub async fn all(db: &DatabaseConnection) -> anyhow::Result<HashMap<String, String>> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare("SELECT key, value FROM config_settings")?;
                let iter = stmt.query_map(params![], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;

                let mut map = HashMap::new();
                for pair in iter {
                    let (k, v) = pair?;
                    map.insert(k, v);
                }
                Ok::<_, rusqlite::Error>(map)
            })
            .await
            .context("failed to get all config settings")?)
    }

    pub async fn set(db: &DatabaseConnection, key: &str, value: &str) -> anyhow::Result<()> {
        let key = key.to_string();
        let value = value.to_string();

        let updated_at: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        db.interact(move |c| {
            c.execute(
                "INSERT OR REPLACE INTO config_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![key, value, updated_at],
            )?;
            Ok(())
        })
        .await
        .context("failed to set config setting")?;

        Ok(())
    }

    pub async fn batch_set(db: &DatabaseConnection, entries: Vec<(String, String)>) -> anyhow::Result<()> {
        let updated_at: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        db.interact(move |c| {
            let tx = c.transaction()?;
            {
                let mut stmt =
                    tx.prepare("INSERT OR REPLACE INTO config_settings (key, value, updated_at) VALUES (?1, ?2, ?3)")?;
                for (key, value) in &entries {
                    stmt.execute(params![key, value, updated_at])?;
                }
            }
            tx.commit()?;
            Ok::<_, rusqlite::Error>(())
        })
        .await
        .context("failed to batch set config settings")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_test_db;

    #[tokio::test]
    async fn test_set_and_get() {
        let db = setup_test_db().await.unwrap();

        ConfigSetting::set(&db, "dns.timeout", "5000").await.unwrap();

        let value = ConfigSetting::get(&db, "dns.timeout").await.unwrap();
        assert_eq!(value, Some("5000".to_string()));
    }

    #[tokio::test]
    async fn test_get_missing_key() {
        let db = setup_test_db().await.unwrap();

        let value = ConfigSetting::get(&db, "nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_set_overwrites_value() {
        let db = setup_test_db().await.unwrap();

        ConfigSetting::set(&db, "dns.timeout", "3000").await.unwrap();
        ConfigSetting::set(&db, "dns.timeout", "5000").await.unwrap();

        let value = ConfigSetting::get(&db, "dns.timeout").await.unwrap();
        assert_eq!(value, Some("5000".to_string()));
    }

    #[tokio::test]
    async fn test_all_empty() {
        let db = setup_test_db().await.unwrap();

        let map = ConfigSetting::all(&db).await.unwrap();
        assert!(map.is_empty());
    }

    #[tokio::test]
    async fn test_all_returns_all_settings() {
        let db = setup_test_db().await.unwrap();

        ConfigSetting::set(&db, "dns.timeout", "3000").await.unwrap();
        ConfigSetting::set(&db, "dns.active", "forwarder").await.unwrap();
        ConfigSetting::set(&db, "dns.forwarder.upstreams", "[\"1.1.1.1\"]")
            .await
            .unwrap();

        let map = ConfigSetting::all(&db).await.unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map["dns.timeout"], "3000");
        assert_eq!(map["dns.active"], "forwarder");
        assert_eq!(map["dns.forwarder.upstreams"], "[\"1.1.1.1\"]");
    }

    #[tokio::test]
    async fn test_set_updates_updated_at() {
        let db = setup_test_db().await.unwrap();

        let before: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        ConfigSetting::set(&db, "dns.timeout", "3000").await.unwrap();

        let after: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let updated_at: i64 = db
            .interact(|c| {
                c.query_row(
                    "SELECT updated_at FROM config_settings WHERE key = ?1",
                    params!["dns.timeout"],
                    |r| r.get::<_, i64>(0),
                )
            })
            .await
            .unwrap();

        assert!(updated_at >= before);
        assert!(updated_at <= after);
    }

    #[tokio::test]
    async fn test_batch_set() {
        let db = setup_test_db().await.unwrap();

        let entries = vec![
            ("dns.timeout".to_string(), "3000".to_string()),
            ("dns.active".to_string(), "forwarder".to_string()),
            ("dns.forwarder.upstreams".to_string(), "[]".to_string()),
        ];

        ConfigSetting::batch_set(&db, entries).await.unwrap();

        let map = ConfigSetting::all(&db).await.unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map["dns.timeout"], "3000");
        assert_eq!(map["dns.active"], "forwarder");
        assert_eq!(map["dns.forwarder.upstreams"], "[]");
    }

    #[tokio::test]
    async fn test_batch_set_overwrites() {
        let db = setup_test_db().await.unwrap();

        ConfigSetting::set(&db, "dns.timeout", "3000").await.unwrap();

        let entries = vec![
            ("dns.timeout".to_string(), "5000".to_string()),
            ("dns.active".to_string(), "forwarder".to_string()),
        ];

        ConfigSetting::batch_set(&db, entries).await.unwrap();

        let value = ConfigSetting::get(&db, "dns.timeout").await.unwrap();
        assert_eq!(value, Some("5000".to_string()));
    }
}
