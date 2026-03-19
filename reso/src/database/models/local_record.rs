use crate::utils::now_millis;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::database::{CoreDatabasePool, DatabaseError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRecord {
    pub id: i64,
    pub name: String,
    pub record_type: u16,
    pub value: String,
    pub ttl: u32,
    pub enabled: bool,
    pub created_at: i64,
}

impl LocalRecord {
    pub fn new(name: String, record_type: u16, value: String, ttl: u32) -> Self {
        Self {
            id: 0,
            name,
            record_type,
            value,
            ttl,
            enabled: true,
            created_at: now_millis(),
        }
    }

    pub async fn insert(&self, db: &CoreDatabasePool) -> Result<(), DatabaseError> {
        let name = self.name.clone();
        let record_type = self.record_type;
        let value = self.value.clone();
        let ttl = self.ttl;
        let created_at = self.created_at;
        db.interact(move |c| {
            c.execute(
                "INSERT INTO local_records (name, record_type, value, ttl, enabled, created_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5)
                 ",
                params![name, record_type, value, ttl, created_at],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    pub async fn delete(db: &CoreDatabasePool, id: i64) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| Ok(c.execute("DELETE FROM local_records WHERE id = ?1", params![id])?))
            .await?;
        Ok(rows > 0)
    }

    pub async fn toggle(db: &CoreDatabasePool, id: i64) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| {
                Ok(c.execute(
                    "UPDATE local_records SET enabled = NOT enabled WHERE id = ?1",
                    params![id],
                )?)
            })
            .await?;
        Ok(rows > 0)
    }

    pub async fn list(db: &CoreDatabasePool, limit: i64, offset: i64) -> Result<Vec<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, record_type, value, ttl, enabled, created_at FROM local_records ORDER BY created_at LIMIT ?1 OFFSET ?2",
                )?;
                let iter = stmt.query_map(params![limit, offset], Self::from_row)?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?)
    }

    pub async fn list_all(db: &CoreDatabasePool) -> Result<Vec<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, record_type, value, ttl, enabled, created_at FROM local_records ORDER BY created_at",
                )?;
                let iter = stmt.query_map([], Self::from_row)?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?)
    }

    pub async fn row_count(db: &CoreDatabasePool) -> Result<i64, DatabaseError> {
        Ok(db
            .interact(|c| c.query_row("SELECT COUNT(*) FROM local_records", [], |r| r.get(0)))
            .await?)
    }

    fn from_row(r: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(LocalRecord {
            id: r.get(0)?,
            name: r.get(1)?,
            record_type: r.get(2)?,
            value: r.get(3)?,
            ttl: r.get(4)?,
            enabled: r.get(5)?,
            created_at: r.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_core_test_db;

    #[tokio::test]
    async fn test_insert_and_list() {
        let db = setup_core_test_db().await.unwrap();
        let record = LocalRecord::new("myapp.home".to_string(), 1, "192.168.1.10".to_string(), 300);
        record.insert(&db.conn).await.unwrap();

        let records = LocalRecord::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "myapp.home");
        assert_eq!(records[0].record_type, 1);
        assert_eq!(records[0].value, "192.168.1.10");
        assert_eq!(records[0].ttl, 300);
        assert!(records[0].enabled);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_core_test_db().await.unwrap();
        LocalRecord::new("myapp.home".to_string(), 1, "192.168.1.10".to_string(), 300)
            .insert(&db.conn)
            .await
            .unwrap();

        let records = LocalRecord::list(&db.conn, 10, 0).await.unwrap();
        let id = records[0].id;

        LocalRecord::delete(&db.conn, id).await.unwrap();
        let records = LocalRecord::list(&db.conn, 10, 0).await.unwrap();
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn test_toggle() {
        let db = setup_core_test_db().await.unwrap();
        LocalRecord::new("myapp.home".to_string(), 1, "192.168.1.10".to_string(), 300)
            .insert(&db.conn)
            .await
            .unwrap();

        let records = LocalRecord::list(&db.conn, 10, 0).await.unwrap();
        let id = records[0].id;
        assert!(records[0].enabled);

        LocalRecord::toggle(&db.conn, id).await.unwrap();
        let records = LocalRecord::list(&db.conn, 10, 0).await.unwrap();
        assert!(!records[0].enabled);

        LocalRecord::toggle(&db.conn, id).await.unwrap();
        let records = LocalRecord::list(&db.conn, 10, 0).await.unwrap();
        assert!(records[0].enabled);
    }

    #[tokio::test]
    async fn test_row_count() {
        let db = setup_core_test_db().await.unwrap();
        assert_eq!(LocalRecord::row_count(&db.conn).await.unwrap(), 0);

        LocalRecord::new("a.home".to_string(), 1, "1.1.1.1".to_string(), 60)
            .insert(&db.conn)
            .await
            .unwrap();
        LocalRecord::new("b.home".to_string(), 1, "2.2.2.2".to_string(), 60)
            .insert(&db.conn)
            .await
            .unwrap();

        assert_eq!(LocalRecord::row_count(&db.conn).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_unique_constraint() {
        let db = setup_core_test_db().await.unwrap();
        LocalRecord::new("myapp.home".to_string(), 1, "192.168.1.10".to_string(), 300)
            .insert(&db.conn)
            .await
            .unwrap();

        let result = LocalRecord::new("myapp.home".to_string(), 1, "192.168.1.10".to_string(), 300)
            .insert(&db.conn)
            .await;
        assert!(result.is_err());
    }
}
