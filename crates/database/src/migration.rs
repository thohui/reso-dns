use std::collections::HashSet;

use turso::Connection;

/// A database migration.
pub struct Migration {
    pub version: i64,
    pub sql: &'static str,
}

/// Run database migrations.
pub async fn run_migrations(conn: &Connection, migrations: &[Migration]) -> anyhow::Result<()> {
    // ensure the schema_migrations table exists
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY
        );
        "#,
        (),
    )
    .await?;

    // load existing migrations
    let mut rows = conn.query("SELECT version FROM schema_migrations", ()).await?;
    let mut applied = HashSet::new();

    while let Some(row) = rows.next().await? {
        let version: i64 = row.get(0)?;
        applied.insert(version);
    }

    // apply pending migrations
    for m in migrations {
        if applied.contains(&m.version) {
            continue;
        }

        tracing::info!("Applying migration {}", m.version);

        // A migration file may contain multiple statements
        conn.execute_batch(m.sql).await?;

        conn.execute("INSERT INTO schema_migrations (version) VALUES (?)", (m.version,))
            .await?;
    }

    Ok(())
}
