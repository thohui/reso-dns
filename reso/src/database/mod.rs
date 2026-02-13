use include_dir::{Dir, include_dir};
use rusqlite_migration::MigrationsBuilder;
use tokio_rusqlite::{Connection, OpenFlags};

pub mod models;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub struct DatabaseConnection(Connection);

impl DatabaseConnection {
    pub async fn conn(&self) -> &Connection {
        &self.0
    }
}

pub async fn connect(db_path: &str) -> anyhow::Result<DatabaseConnection> {
    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .await?;

    connection
        .call(|c| {
            c.execute_batch(
                r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        "#,
            )
        })
        .await?;

    Ok(DatabaseConnection(connection))
}

pub async fn run_migrations(connection: &DatabaseConnection) -> anyhow::Result<()> {
    let migrations = MigrationsBuilder::from_directory(&MIGRATIONS_DIR)?.finalize();
    let conn = connection.conn().await;
    conn.call(move |c| migrations.to_latest(c)).await?;
    Ok(())
}

#[cfg(test)]
async fn setup_test_db() -> anyhow::Result<DatabaseConnection> {
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
    pub async fn test_test_database() {
        setup_test_db().await.unwrap();
    }
}
