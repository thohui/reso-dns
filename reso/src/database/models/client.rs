use std::collections::HashMap;

use rusqlite::params;
use serde::Serialize;

use crate::{
    database::{CoreDatabasePool, DatabaseError, query::WhereBuilder},
    reverse::HostnameCheck,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Client {
    pub ip: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub hostname: Option<String>,
    pub hostname_checked_at: Option<i64>,
    pub display_name: Option<String>,
}

impl Client {
    fn from_row(r: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Client {
            ip: r.get(0)?,
            first_seen_at: r.get(1)?,
            last_seen_at: r.get(2)?,
            hostname: r.get(3)?,
            hostname_checked_at: r.get(4)?,
            display_name: r.get(5)?,
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

const SEARCH_FILTER: &str = "?1 IS NULL
     OR ip LIKE ?1 ESCAPE '\\'
     OR hostname LIKE ?1 ESCAPE '\\'
     OR display_name LIKE ?1 ESCAPE '\\'";

fn search_pattern(search: Option<String>) -> Option<String> {
    search.map(|s| format!("%{}%", WhereBuilder::escape_like(&s)))
}

pub async fn list(
    db: &CoreDatabasePool,
    limit: i64,
    offset: i64,
    search: Option<String>,
) -> Result<Vec<Client>, DatabaseError> {
    let pattern = search_pattern(search);
    db.interact(move |c| {
        let mut stmt = c.prepare(&format!(
            "SELECT ip, first_seen_at, last_seen_at, hostname, hostname_checked_at, display_name
             FROM clients WHERE {SEARCH_FILTER}
             ORDER BY last_seen_at DESC, ip LIMIT ?2 OFFSET ?3"
        ))?;
        let iter = stmt.query_map(params![pattern, limit, offset], Client::from_row)?;
        iter.collect::<rusqlite::Result<Vec<_>>>()
    })
    .await
}

pub async fn due_for_hostname_check(
    db: &CoreDatabasePool,
    checked_before_ms: i64,
    limit: i64,
) -> Result<Vec<String>, DatabaseError> {
    db.interact(move |c| {
        // NULLs sort first in ascending order
        let mut stmt = c.prepare(
            "SELECT ip FROM clients
             WHERE hostname_checked_at IS NULL OR hostname_checked_at < ?1
             ORDER BY hostname_checked_at, last_seen_at DESC
             LIMIT ?2",
        )?;
        let iter = stmt.query_map(params![checked_before_ms, limit], |r| r.get(0))?;
        iter.collect::<rusqlite::Result<Vec<_>>>()
    })
    .await
}

pub async fn batch_set_hostnames(
    db: &CoreDatabasePool,
    checks: Vec<(String, HostnameCheck)>,
    checked_at: i64,
    not_found_checked_at: i64,
    failed_checked_at: i64,
) -> Result<(), DatabaseError> {
    if checks.is_empty() {
        return Ok(());
    }

    db.interact(move |c| {
        let tx = c.transaction()?;
        {
            let mut set = tx.prepare("UPDATE clients SET hostname = ?2, hostname_checked_at = ?3 WHERE ip = ?1")?;
            let mut touch = tx.prepare("UPDATE clients SET hostname_checked_at = ?2 WHERE ip = ?1")?;
            for (ip, check) in &checks {
                match check {
                    HostnameCheck::Found(hostname) => {
                        set.execute(params![ip, hostname, checked_at])?;
                    }
                    HostnameCheck::NotFound => {
                        set.execute(params![ip, None::<String>, not_found_checked_at])?;
                    }
                    HostnameCheck::Skipped => {
                        set.execute(params![ip, None::<String>, checked_at])?;
                    }
                    HostnameCheck::Failed => {
                        touch.execute(params![ip, failed_checked_at])?;
                    }
                }
            }
        }
        tx.commit()?;
        Ok(())
    })
    .await
}

pub async fn reset_hostname_checks(db: &CoreDatabasePool, clear_hostnames: bool) -> Result<usize, DatabaseError> {
    let sql = if clear_hostnames {
        "UPDATE clients SET hostname = NULL, hostname_checked_at = NULL"
    } else {
        "UPDATE clients SET hostname_checked_at = NULL"
    };
    db.interact(move |c| c.execute(sql, [])).await
}

pub async fn set_display_name(
    db: &CoreDatabasePool,
    ip: String,
    display_name: Option<String>,
) -> Result<bool, DatabaseError> {
    db.interact(move |c| {
        c.execute(
            "UPDATE clients SET display_name = ?2 WHERE ip = ?1",
            params![ip, display_name],
        )
        .map(|n| n > 0)
    })
    .await
}

pub async fn row_count(db: &CoreDatabasePool, search: Option<String>) -> Result<i64, DatabaseError> {
    let pattern = search_pattern(search);
    db.interact(move |c| {
        c.query_row(
            &format!("SELECT COUNT(*) FROM clients WHERE {SEARCH_FILTER}"),
            params![pattern],
            |r| r.get(0),
        )
    })
    .await
}

pub async fn names_for(db: &CoreDatabasePool, mut ips: Vec<String>) -> Result<HashMap<String, String>, DatabaseError> {
    ips.sort_unstable();
    ips.dedup();
    if ips.is_empty() {
        return Ok(HashMap::new());
    }

    db.interact(move |c| {
        let placeholders = vec!["?"; ips.len()].join(",");
        let mut stmt = c.prepare(&format!(
            "SELECT ip, display_name, hostname FROM clients
             WHERE ip IN ({placeholders}) AND COALESCE(display_name, hostname) IS NOT NULL"
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(&ips), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut names = HashMap::new();
        for row in rows {
            let (ip, display_name, hostname) = row?;
            let short_hostname = hostname.and_then(|h| h.split('.').next().map(str::to_string));
            if let Some(name) = display_name.or(short_hostname) {
                names.insert(ip, name);
            }
        }
        Ok(names)
    })
    .await
}

pub async fn delete_before(db: &CoreDatabasePool, cutoff_ms: i64) -> Result<usize, DatabaseError> {
    db.interact(move |c| {
        c.execute(
            "DELETE FROM clients WHERE last_seen_at < ?1 AND display_name IS NULL",
            params![cutoff_ms],
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_core_test_db;

    fn found(hostname: &str) -> HostnameCheck {
        HostnameCheck::Found(hostname.into())
    }

    #[tokio::test]
    async fn test_due_for_hostname_check_unchecked_first_then_oldest() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![
                ("10.0.0.1".into(), 100),
                ("10.0.0.2".into(), 100),
                ("10.0.0.3".into(), 100),
                ("10.0.0.4".into(), 100),
            ],
        )
        .await
        .unwrap();
        batch_set_hostnames(
            &db.conn,
            vec![("10.0.0.1".into(), HostnameCheck::Failed)],
            500,
            500,
            500,
        )
        .await
        .unwrap();
        batch_set_hostnames(
            &db.conn,
            vec![("10.0.0.2".into(), HostnameCheck::Failed)],
            300,
            300,
            300,
        )
        .await
        .unwrap();
        batch_set_hostnames(
            &db.conn,
            vec![("10.0.0.3".into(), HostnameCheck::Failed)],
            1000,
            1000,
            1000,
        )
        .await
        .unwrap();

        let due = due_for_hostname_check(&db.conn, 900, 10).await.unwrap();
        assert_eq!(due, vec!["10.0.0.4", "10.0.0.2", "10.0.0.1"]);

        let limited = due_for_hostname_check(&db.conn, 900, 1).await.unwrap();
        assert_eq!(limited, vec!["10.0.0.4"]);
    }

    #[tokio::test]
    async fn test_set_hostnames_found_not_found_failed() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100), ("10.0.0.2".into(), 100)])
            .await
            .unwrap();
        batch_set_hostnames(
            &db.conn,
            vec![
                ("10.0.0.1".into(), found("laptop.lan")),
                ("10.0.0.2".into(), found("phone.lan")),
            ],
            200,
            200,
            200,
        )
        .await
        .unwrap();

        // a failed lookup keeps the old name, an answered one without a name clears it
        batch_set_hostnames(
            &db.conn,
            vec![
                ("10.0.0.1".into(), HostnameCheck::Failed),
                ("10.0.0.2".into(), HostnameCheck::NotFound),
            ],
            300,
            300,
            300,
        )
        .await
        .unwrap();

        let clients = list(&db.conn, 10, 0, None).await.unwrap();
        let laptop = clients.iter().find(|c| c.ip == "10.0.0.1").unwrap();
        assert_eq!(laptop.hostname.as_deref(), Some("laptop.lan"));
        assert_eq!(laptop.hostname_checked_at, Some(300));

        let phone = clients.iter().find(|c| c.ip == "10.0.0.2").unwrap();
        assert_eq!(phone.hostname, None);
        assert_eq!(phone.hostname_checked_at, Some(300));
    }

    #[tokio::test]
    async fn test_upsert_keeps_first_seen_and_moves_last_seen() {
        let db = setup_core_test_db().await.unwrap();
        let later = 100 + LAST_SEEN_UPDATE_INTERVAL_MS;
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100)]).await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), later)]).await.unwrap();

        let clients = list(&db.conn, 10, 0, None).await.unwrap();
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

        let clients = list(&db.conn, 10, 0, None).await.unwrap();
        assert_eq!(clients[0].last_seen_at, 100);
    }

    #[tokio::test]
    async fn test_upsert_older_timestamp_keeps_range() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 300)]).await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100)]).await.unwrap();

        let clients = list(&db.conn, 10, 0, None).await.unwrap();
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

        let ips: Vec<String> = list(&db.conn, 10, 0, None)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.ip)
            .collect();
        assert_eq!(ips, vec!["10.0.0.2", "10.0.0.3", "10.0.0.1"]);

        let page = list(&db.conn, 1, 1, None).await.unwrap();
        assert_eq!(page[0].ip, "10.0.0.3");
    }

    #[tokio::test]
    async fn test_search_matches_ip_hostname_and_display_name() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![
                ("192.168.1.10".into(), 300),
                ("192.168.1.20".into(), 200),
                ("10.0.0.5".into(), 100),
            ],
        )
        .await
        .unwrap();
        batch_set_hostnames(
            &db.conn,
            vec![("192.168.1.20".into(), found("printer.lan"))],
            400,
            400,
            400,
        )
        .await
        .unwrap();
        set_display_name(&db.conn, "10.0.0.5".into(), Some("Living room TV".into()))
            .await
            .unwrap();

        let ips = |clients: Vec<Client>| clients.into_iter().map(|c| c.ip).collect::<Vec<_>>();

        assert_eq!(
            ips(list(&db.conn, 10, 0, Some("192.168".into())).await.unwrap()),
            vec!["192.168.1.10", "192.168.1.20"]
        );
        assert_eq!(
            ips(list(&db.conn, 10, 0, Some("printer".into())).await.unwrap()),
            vec!["192.168.1.20"]
        );
        assert_eq!(
            ips(list(&db.conn, 10, 0, Some("tv".into())).await.unwrap()),
            vec!["10.0.0.5"]
        );
        assert_eq!(row_count(&db.conn, Some("192.168".into())).await.unwrap(), 2);
        assert_eq!(row_count(&db.conn, Some("%".into())).await.unwrap(), 0);
        assert_eq!(row_count(&db.conn, None).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_hostname_checks_store_checked_at_per_result() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![
                ("10.0.0.1".into(), 100),
                ("10.0.0.2".into(), 100),
                ("10.0.0.3".into(), 100),
                ("8.8.8.8".into(), 100),
            ],
        )
        .await
        .unwrap();
        batch_set_hostnames(&db.conn, vec![("8.8.8.8".into(), found("dns.google"))], 200, 200, 200)
            .await
            .unwrap();

        batch_set_hostnames(
            &db.conn,
            vec![
                ("10.0.0.1".into(), found("laptop.lan")),
                ("10.0.0.2".into(), HostnameCheck::NotFound),
                ("10.0.0.3".into(), HostnameCheck::Failed),
                ("8.8.8.8".into(), HostnameCheck::Skipped),
            ],
            1000,
            400,
            300,
        )
        .await
        .unwrap();

        let clients = list(&db.conn, 10, 0, None).await.unwrap();
        let client = |ip: &str| clients.iter().find(|c| c.ip == ip).unwrap();
        assert_eq!(client("10.0.0.1").hostname_checked_at, Some(1000));
        assert_eq!(client("10.0.0.2").hostname_checked_at, Some(400));
        assert_eq!(client("10.0.0.3").hostname_checked_at, Some(300));
        assert_eq!(client("8.8.8.8").hostname_checked_at, Some(1000));
        assert_eq!(client("8.8.8.8").hostname, None);

        let due = due_for_hostname_check(&db.conn, 500, 10).await.unwrap();
        assert_eq!(due, vec!["10.0.0.3", "10.0.0.2"]);
    }

    #[tokio::test]
    async fn test_reset_hostname_checks() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(&db.conn, vec![("10.0.0.1".into(), 100)]).await.unwrap();
        batch_set_hostnames(&db.conn, vec![("10.0.0.1".into(), found("laptop.lan"))], 200, 200, 200)
            .await
            .unwrap();

        reset_hostname_checks(&db.conn, false).await.unwrap();
        let clients = list(&db.conn, 10, 0, None).await.unwrap();
        assert_eq!(clients[0].hostname.as_deref(), Some("laptop.lan"));
        assert_eq!(clients[0].hostname_checked_at, None);

        reset_hostname_checks(&db.conn, true).await.unwrap();
        let clients = list(&db.conn, 10, 0, None).await.unwrap();
        assert_eq!(clients[0].hostname, None);
    }

    #[tokio::test]
    async fn test_delete_before_keeps_named_clients() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![
                ("10.0.0.1".into(), 100),
                ("10.0.0.2".into(), 100),
                ("10.0.0.3".into(), 300),
            ],
        )
        .await
        .unwrap();
        set_display_name(&db.conn, "10.0.0.2".into(), Some("Living room TV".into()))
            .await
            .unwrap();

        assert_eq!(delete_before(&db.conn, 200).await.unwrap(), 1);

        let ips: Vec<String> = list(&db.conn, 10, 0, None)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.ip)
            .collect();
        assert_eq!(ips, vec!["10.0.0.3", "10.0.0.2"]);
    }

    #[tokio::test]
    async fn test_names_for_prefers_display_name() {
        let db = setup_core_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![
                ("10.0.0.1".into(), 100),
                ("10.0.0.2".into(), 100),
                ("10.0.0.3".into(), 100),
            ],
        )
        .await
        .unwrap();
        batch_set_hostnames(
            &db.conn,
            vec![
                ("10.0.0.1".into(), found("laptop.lan")),
                ("10.0.0.2".into(), found("tv.lan")),
            ],
            200,
            200,
            200,
        )
        .await
        .unwrap();
        set_display_name(&db.conn, "10.0.0.2".into(), Some("Mr. Smith's TV".into()))
            .await
            .unwrap();

        let names = names_for(
            &db.conn,
            vec![
                "10.0.0.1".into(),
                "10.0.0.2".into(),
                "10.0.0.2".into(),
                "10.0.0.3".into(),
                "10.0.0.9".into(),
            ],
        )
        .await
        .unwrap();

        assert_eq!(
            names,
            HashMap::from([
                ("10.0.0.1".to_string(), "laptop".to_string()),
                ("10.0.0.2".to_string(), "Mr. Smith's TV".to_string()),
            ])
        );
        assert!(names_for(&db.conn, vec![]).await.unwrap().is_empty());
    }
}
