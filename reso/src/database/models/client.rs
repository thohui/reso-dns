use rusqlite::params;
use serde::Serialize;

use crate::database::{CoreDatabasePool, DatabaseError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Client {
    pub ip: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}

impl Client {
    fn from_row(r: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Client {
            ip: r.get(0)?,
            first_seen_at: r.get(1)?,
            last_seen_at: r.get(2)?,
        })
    }
}

pub const LAST_SEEN_UPDATE_INTERVAL_MS: i64 = 60_000;

pub async fn batch_upsert(db: &CoreDatabasePool, seen: Vec<(String, i64)>) -> Result<(), DatabaseError> {
    if seen.is_empty() {
        return Ok(());
    }

    db.interact(move |c| {
        let tx = c.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO clients (ip, first_seen_at, last_seen_at)
                 VALUES (?1, ?2, ?2)
                 ON CONFLICT(ip) DO UPDATE SET
                     first_seen_at = MIN(first_seen_at, excluded.first_seen_at),
                     last_seen_at = MAX(last_seen_at, excluded.last_seen_at)
                 WHERE excluded.last_seen_at >= clients.last_seen_at + ?3
                    OR excluded.first_seen_at < clients.first_seen_at",
            )?;
            for (ip, ts) in &seen {
                stmt.execute(params![ip, ts, LAST_SEEN_UPDATE_INTERVAL_MS])?;
            }
        }
        tx.commit()?;
        Ok(())
    })
    .await
}

pub async fn list(db: &CoreDatabasePool, limit: i64, offset: i64) -> Result<Vec<Client>, DatabaseError> {
    db.interact(move |c| {
        let mut stmt = c.prepare(
            "SELECT ip, first_seen_at, last_seen_at FROM clients ORDER BY last_seen_at DESC, ip LIMIT ?1 OFFSET ?2",
        )?;
        let iter = stmt.query_map(params![limit, offset], Client::from_row)?;
        iter.collect::<rusqlite::Result<Vec<_>>>()
    })
    .await
}

pub async fn row_count(db: &CoreDatabasePool) -> Result<i64, DatabaseError> {
    db.interact(|c| c.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0)))
        .await
}

pub async fn delete_before(db: &CoreDatabasePool, cutoff_ms: i64) -> Result<usize, DatabaseError> {
    db.interact(move |c| c.execute("DELETE FROM clients WHERE last_seen_at < ?1", params![cutoff_ms]))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_core_test_db;

    #[tokio::test]
    async fn test_upsert_inserts_new_clients() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100), ("10.0.0.2".into(), 200)])
            .await
            .unwrap();

        assert_eq!(row_count(&db.conn).await.unwrap(), 2);
        let clients = list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(
            clients[0],
            Client {
                ip: "10.0.0.2".into(),
                first_seen_at: 200,
                last_seen_at: 200
            }
        );
    }

    #[tokio::test]
    async fn test_upsert_keeps_first_seen_and_moves_last_seen() {
        let db = setup_core_test_db().await.unwrap();
        let later = 100 + LAST_SEEN_UPDATE_INTERVAL_MS;
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100)]).await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), later)]).await.unwrap();

        let clients = list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].first_seen_at, 100);
        assert_eq!(clients[0].last_seen_at, later);
    }

    #[tokio::test]
    async fn test_upsert_skips_small_last_seen_moves() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100)]).await.unwrap();
        batch_upsert(
            &db.conn,
            vec![("10.0.0.1".into(), 100 + LAST_SEEN_UPDATE_INTERVAL_MS - 1)],
        )
        .await
        .unwrap();

        let clients = list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(clients[0].last_seen_at, 100);
    }

    #[tokio::test]
    async fn test_upsert_older_timestamp_keeps_range() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 300)]).await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100)]).await.unwrap();

        let clients = list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(clients[0].first_seen_at, 100);
        assert_eq!(clients[0].last_seen_at, 300);
    }

    #[tokio::test]
    async fn test_list_ordered_by_last_seen() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![
                ("10.0.0.1".into(), 100),
                ("10.0.0.2".into(), 300),
                ("10.0.0.3".into(), 200),
            ],
        )
        .await
        .unwrap();

        let ips: Vec<String> = list(&db.conn, 10, 0).await.unwrap().into_iter().map(|c| c.ip).collect();
        assert_eq!(ips, vec!["10.0.0.2", "10.0.0.3", "10.0.0.1"]);

        let page = list(&db.conn, 1, 1).await.unwrap();
        assert_eq!(page[0].ip, "10.0.0.3");
    }

    #[tokio::test]
    async fn test_delete_before_removes_stale_clients() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100), ("10.0.0.2".into(), 300)])
            .await
            .unwrap();

        assert_eq!(delete_before(&db.conn, 200).await.unwrap(), 1);

        let clients = list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].ip, "10.0.0.2");
    }
}
