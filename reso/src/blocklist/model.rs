use async_trait::async_trait;
use reso_database::DatabaseOperations;
use reso_dns::domain_name::DomainName;
use turso::{Connection, params};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockedDomain(pub DomainName);

impl BlockedDomain {
    pub fn new(domain: DomainName) -> Self {
        BlockedDomain(domain)
    }

    pub fn domain(&self) -> &str {
        &self.0
    }
}

#[async_trait]
impl DatabaseOperations for BlockedDomain {
    type PrimaryKey = DomainName;

    async fn create(&self, connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute(
                "INSERT INTO blocklist (domain) VALUES (?1)",
                params![self.domain()],
            )
            .await?;
        Ok(())
    }

    async fn get(db: &Connection, key: &Self::PrimaryKey) -> anyhow::Result<Option<Self>> {
        let mut rows = db
            .query(
                "SELECT domain FROM blocklist WHERE domain = ?1 LIMIT 1",
                params![key.as_str()],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let domain: String = row.get(0)?;
            let qname = DomainName::from_ascii(domain)?;
            Ok(Some(BlockedDomain(qname)))
        } else {
            Ok(None)
        }
    }

    async fn update(&self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE blocklist SET domain = ?1 WHERE domain = ?2",
            params![self.domain(), self.domain()],
        )
        .await?;
        Ok(())
    }

    async fn delete(db: &Connection, key: &Self::PrimaryKey) -> anyhow::Result<()> {
        db.execute(
            "DELETE FROM blocklist WHERE domain = ?1",
            params![key.as_str()],
        )
        .await?;
        Ok(())
    }

    async fn all(connection: &Connection) -> anyhow::Result<Vec<Self>> {
        let mut out = Vec::new();
        let mut rows = connection.query("SELECT domain FROM blocklist", ()).await?;

        while let Some(row) = rows.next().await? {
            let domain: String = row.get(0)?;
            let qname = DomainName::from_ascii(domain)?;
            out.push(BlockedDomain(qname));
        }

        Ok(out)
    }
}
