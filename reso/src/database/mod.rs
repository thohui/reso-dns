use std::marker::PhantomData;

use deadpool_sqlite::{Config, Hook, HookError, Pool, Runtime};
use include_dir::{Dir, include_dir};
use rusqlite_migration::MigrationsBuilder;

pub mod models;

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

impl DatabaseError {
    pub fn is_unique_constraint_violation(&self) -> bool {
        matches!(self, DatabaseError::Query(e) if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation))
    }

    pub fn is_no_rows_returned(&self) -> bool {
        matches!(self, DatabaseError::Query(e) if *e == rusqlite::Error::QueryReturnedNoRows)
    }
}

impl From<deadpool_sqlite::InteractError> for DatabaseError {
    fn from(e: deadpool_sqlite::InteractError) -> Self {
        DatabaseError::Interact(e.to_string())
    }
}

pub struct DatabasePool<T> {
    pool: Pool,
    _marker: PhantomData<T>,
}

impl<T> DatabasePool<T> {
    pub async fn conn(&self) -> Result<deadpool_sqlite::Object, DatabaseError> {
        Ok(self.pool.get().await?)
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

pub struct CoreDb;
pub struct MetricsDb;

pub type CoreDatabasePool = DatabasePool<CoreDb>;
pub type MetricsDatabasePool = DatabasePool<MetricsDb>;

pub async fn connect_core_db(db_path: &str) -> anyhow::Result<CoreDatabasePool> {
    let pool = Config::new(db_path)
        .builder(Runtime::Tokio1)?
        .max_size(DB_POOL_SIZE)
        .post_create(Hook::async_fn(|conn, _| {
            Box::pin(async move {
                conn.interact(|c| {
                    c.execute_batch(
                        r#"
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA foreign_keys = ON;
                PRAGMA busy_timeout = 5000;
                PRAGMA wal_autocheckpoint = 1000;
                PRAGMA optimize=0x10002;
                "#,
                    )
                })
                .await
                .map_err(|e| HookError::message(e.to_string()))?
                .map_err(HookError::Backend)
            })
        }))
        .build()?;

    Ok(CoreDatabasePool {
        pool,
        _marker: PhantomData,
    })
}

pub async fn connect_metrics_db(db_path: &str) -> anyhow::Result<MetricsDatabasePool> {
    let pool = Config::new(db_path)
        .builder(Runtime::Tokio1)?
        .max_size(DB_POOL_SIZE)
        .post_create(Hook::async_fn(|conn, _| {
            Box::pin(async move {
                conn.interact(|c| {
                    c.execute_batch(
                        r#"
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA foreign_keys = ON;
                PRAGMA busy_timeout = 5000;
                PRAGMA wal_autocheckpoint = 200;
                PRAGMA optimize=0x10002;
                "#,
                    )
                })
                .await
                .map_err(|e| HookError::message(e.to_string()))?
                .map_err(HookError::Backend)
            })
        }))
        .build()?;

    Ok(MetricsDatabasePool {
        pool,
        _marker: PhantomData,
    })
}

static CORE_MIGRATIONS: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");
static METRICS_MIGRATIONS: Dir = include_dir!("$CARGO_MANIFEST_DIR/metrics_migrations");

pub async fn run_core_db_migrations(connection: &CoreDatabasePool) -> anyhow::Result<()> {
    let migrations = MigrationsBuilder::from_directory(&CORE_MIGRATIONS)?.finalize();
    let conn = connection.conn().await?;
    conn.interact(move |c| migrations.to_latest(c))
        .await
        .map_err(|e| anyhow::anyhow!("interact error: {}", e))??;
    Ok(())
}

pub async fn run_metrics_db_migrations(connection: &MetricsDatabasePool) -> anyhow::Result<()> {
    let migrations = MigrationsBuilder::from_directory(&METRICS_MIGRATIONS)?.finalize();
    let conn = connection.conn().await?;
    conn.interact(move |c| migrations.to_latest(c))
        .await
        .map_err(|e| anyhow::anyhow!("interact error: {}", e))??;

    Ok(())
}

#[cfg(test)]
use tempfile::NamedTempFile;

#[cfg(test)]
pub struct CoreDbFixture {
    pub conn: CoreDatabasePool,
    _temp_file: NamedTempFile,
}

#[cfg(test)]
pub(crate) async fn setup_core_test_db() -> anyhow::Result<CoreDbFixture> {
    let _temp_file = NamedTempFile::new()?;
    let db_path = _temp_file.path().to_str().unwrap();
    let conn = connect_core_db(db_path).await?;
    run_core_db_migrations(&conn).await?;
    Ok(CoreDbFixture { _temp_file, conn })
}

#[cfg(test)]
pub struct MetricsDbFixture {
    pub conn: MetricsDatabasePool,
    _temp_file: NamedTempFile,
}

#[cfg(test)]
pub(crate) async fn setup_metrics_test_db() -> anyhow::Result<MetricsDbFixture> {
    let _temp_file = NamedTempFile::new()?;
    let db_path = _temp_file.path().to_str().unwrap();
    let conn = connect_metrics_db(db_path).await?;
    run_metrics_db_migrations(&conn).await?;
    Ok(MetricsDbFixture { conn, _temp_file })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn test_database_setup() {
        setup_core_test_db().await.unwrap();
    }
}
