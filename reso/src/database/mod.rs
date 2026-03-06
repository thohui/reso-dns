use deadpool_sqlite::{Config, Pool, Runtime};
use include_dir::{Dir, include_dir};
use rusqlite_migration::MigrationsBuilder;

pub mod models;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

const DB_POOL_SIZE: usize = 5;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("pool error: {0}")]
    Pool(#[from] deadpool_sqlite::PoolError),

    #[error("interact error: {0}")]
    Interact(String),

    #[error("query error: {0}")]
    Query(#[from] rusqlite::Error),
}

impl From<deadpool_sqlite::InteractError> for DatabaseError {
    fn from(e: deadpool_sqlite::InteractError) -> Self {
        DatabaseError::Interact(e.to_string())
    }
}

pub struct DatabaseConnection(Pool);

impl DatabaseConnection {
    pub async fn conn(&self) -> Result<deadpool_sqlite::Object, DatabaseError> {
        Ok(self.0.get().await?)
    }

    pub async fn interact<F, R>(&self, f: F) -> Result<R, DatabaseError>
    where
        F: FnOnce(&mut rusqlite::Connection) -> Result<R, rusqlite::Error> + Send + 'static,
        R: Send + 'static,
    {
        let conn = self.conn().await?;
        let result = conn.interact(f).await??;
        Ok(result)
    }
}

pub async fn connect(db_path: &str) -> anyhow::Result<DatabaseConnection> {
    let pool = Config::new(db_path)
        .builder(Runtime::Tokio1)?
        .max_size(DB_POOL_SIZE)
        .build()?;

    pool.get()
        .await?
        .interact(|c| {
            c.execute_batch(
                r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        "#,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("interact error: {}", e))??;

    Ok(DatabaseConnection(pool))
}

pub async fn run_migrations(connection: &DatabaseConnection) -> anyhow::Result<()> {
    let migrations = MigrationsBuilder::from_directory(&MIGRATIONS_DIR)?.finalize();
    let conn = connection.conn().await?;
    conn.interact(move |c| migrations.to_latest(c))
        .await
        .map_err(|e| anyhow::anyhow!("interact error: {}", e))??;
    Ok(())
}

#[cfg(test)]
pub(crate) async fn setup_test_db() -> anyhow::Result<DatabaseConnection> {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();
    let conn = connect(db_path).await?;
    run_migrations(&conn).await?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn test_database_setup() {
        setup_test_db().await.unwrap();
    }
}
