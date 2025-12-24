use anyhow::Context;
use reso_dns::domain_name::DomainName;
use tokio_rusqlite::{OptionalExtension, Row, params, rusqlite};

use crate::database::DatabaseConnection;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockedDomain(pub DomainName);

impl BlockedDomain {
    pub fn new(domain: DomainName) -> Self {
        Self(domain)
    }
}

impl BlockedDomain {
    pub async fn insert(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = db.conn().await;
        let str = self.0.to_string();
        conn.call(move |c| c.execute("INSERT OR IGNORE INTO blocklist (domain) VALUES (?)", [str]))
            .await?;
        Ok(())
    }

    pub async fn get(db: &DatabaseConnection, domain: &DomainName) -> anyhow::Result<Option<Self>> {
        let conn = db.conn().await;

        let str = domain.to_string();

        let maybe = conn
            .call(move |c| {
                c.query_one("SELECT domain FROM blocklist WHERE domain = ?", params![str], |r| {
                    let domain: String = r.get(0)?;
                    Ok(domain)
                })
                .optional()
            })
            .await?;

        match maybe {
            Some(s) => {
                let qname = DomainName::from_ascii(s).context("parse DomainName from db")?;
                Ok(Some(Self(qname)))
            }
            None => Ok(None),
        }
    }

    pub async fn delete(db: &DatabaseConnection, domain: &DomainName) -> anyhow::Result<()> {
        let conn = db.conn().await;

        let str = domain.to_string();
        conn.call(move |c| c.execute("DELETE FROM blocklist where domain = ?", params![str]))
            .await?;
        Ok(())
    }

    pub async fn list(db: &DatabaseConnection) -> anyhow::Result<Vec<Self>> {
        let conn = db.conn().await;

        let raw: Vec<String> = conn
            .call(|c| -> rusqlite::Result<Vec<String>> {
                let mut stmt = c.prepare("SELECT domain FROM blocklist ORDER BY domain")?;
                let iter = stmt.query_map([], |r| r.get::<_, String>(0))?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        let mut out = Vec::with_capacity(raw.len());
        for s in raw {
            let dn = DomainName::from_ascii(s).context("parse DomainName from db")?;
            out.push(BlockedDomain(dn));
        }

        Ok(out)
    }
}
