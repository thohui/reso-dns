use async_trait::async_trait;
use turso::{Builder, Connection};

mod migration;

#[async_trait]
pub trait DatabaseOperations: Sized {
    type PrimaryKey: Send + Sync + Clone;

    /// Create a record.
    async fn create(&self, db: &Connection) -> anyhow::Result<()>;

    /// Retrieve a record.
    async fn get(db: &Connection, key: &Self::PrimaryKey) -> anyhow::Result<Option<Self>>;

    /// Update a record.
    async fn update(&self, db: &Connection) -> anyhow::Result<()>;

    /// Delete a record.
    async fn delete(db: &Connection, key: &Self::PrimaryKey) -> anyhow::Result<()>;

    /// Retrieve all records.
    async fn all(connection: &Connection) -> anyhow::Result<Vec<Self>>;
}

pub async fn connect(url: &str) -> anyhow::Result<Connection> {
    let db = Builder::new_local(url)
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
    let connection = db.connect()?;
    Ok(connection)
}

pub use migration::{Migration, run_migrations};
