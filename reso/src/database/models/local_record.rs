use anyhow::Context;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::database::CoreDatabasePool;

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
            created_at: Utc::now().timestamp_millis(),
        }
    }

    pub async fn insert(&self, db: &CoreDatabasePool) -> anyhow::Result<()> {
        let name = self.name.clone();
        let record_type = self.record_type;
        let value = self.value.clone();
        let ttl = self.ttl;
        let created_at = self.created_at;
        db.interact(move |c| {
            c.execute(
                "INSERT INTO local_records (name, record_type, value, ttl, enabled, created_at) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                params![name, record_type, value, ttl, created_at],
            )?;
            Ok(())
        })
        .await
        .context("failed to insert local record")?;
        Ok(())
    }

    pub async fn delete(db: &CoreDatabasePool, id: i64) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute("DELETE FROM local_records WHERE id = ?1", params![id])?;
            Ok(())
        })
        .await
        .context("failed to delete local record")?;
        Ok(())
    }

    pub async fn toggle(db: &CoreDatabasePool, id: i64) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute(
                "UPDATE local_records SET enabled = NOT enabled WHERE id = ?1",
                params![id],
            )?;
            Ok(())
        })
        .await
        .context("failed to toggle local record")?;
        Ok(())
    }

    pub async fn list(db: &CoreDatabasePool, limit: i64, offset: i64) -> anyhow::Result<Vec<Self>> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, record_type, value, ttl, enabled, created_at FROM local_records ORDER BY created_at LIMIT ?1 OFFSET ?2",
                )?;
                let iter = stmt.query_map(params![limit, offset], Self::from_row)?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
            .context("failed to list local records")?)
    }

    pub async fn list_all(db: &CoreDatabasePool) -> anyhow::Result<Vec<Self>> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, record_type, value, ttl, enabled, created_at FROM local_records ORDER BY created_at",
                )?;
                let iter = stmt.query_map([], Self::from_row)?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
            .context("failed to list all local records")?)
    }

    pub async fn row_count(db: &CoreDatabasePool) -> anyhow::Result<i64> {
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
