use turso::{Connection, params};

use crate::database::{DatabaseOperations, PrimaryKey};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockedDomain(pub String);

impl BlockedDomain {
    pub fn new(domain: String) -> Self {
        BlockedDomain(domain)
    }

    pub fn domain(&self) -> &str {
        &self.0
    }
}

impl PrimaryKey for String {}

impl DatabaseOperations for BlockedDomain {
    type PrimaryKey = String;

    async fn create(&self, connection: &Connection) -> anyhow::Result<()> {
        // Explicit column is safer than VALUES(?) if schema changes
        connection
            .execute(
                "INSERT INTO blocklist (domain) VALUES (?1)",
                params![self.domain()],
            )
            .await?;
        Ok(())
    }

    // SELECT one
    async fn get(db: &Connection, key: &Self::PrimaryKey) -> anyhow::Result<Option<Self>> {
        let mut rows = db
            .query(
                "SELECT domain FROM blocklist WHERE domain = ?1 LIMIT 1",
                params![key.as_str()],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let domain: String = row.get(0)?;
            Ok(Some(BlockedDomain(domain)))
        } else {
            Ok(None)
        }
    }

    // UPDATE (rename an existing domain key -> current self.domain())
    async fn update(&self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE blocklist SET domain = ?1 WHERE domain = ?2",
            params![self.domain(), self.domain()],
        )
        .await?;
        Ok(())
    }

    // DELETE by key
    async fn delete(db: &Connection, key: &Self::PrimaryKey) -> anyhow::Result<()> {
        db.execute(
            "DELETE FROM blocklist WHERE domain = ?1",
            params![key.as_str()],
        )
        .await?;
        Ok(())
    }

    // SELECT all
    async fn all(connection: &Connection) -> anyhow::Result<Vec<Self>> {
        let mut out = Vec::new();
        let mut rows = connection.query("SELECT domain FROM blocklist", ()).await?;

        while let Some(row) = rows.next().await? {
            let domain: String = row.get(0)?;
            out.push(BlockedDomain(domain));
        }

        Ok(out)
    }
}
