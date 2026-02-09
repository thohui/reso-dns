use tokio_rusqlite::{params, rusqlite};

use crate::database::DatabaseConnection;

pub struct Config {
    id: i8,
    version: i64,
    updated_at: i64,
    data: serde_json::Value,
}

impl Config {
    pub async fn get(conn: &DatabaseConnection) -> anyhow::Result<Self> {
        let conn = conn.conn().await;

        struct ConfigRow {
            id: i8,
            version: i64,
            updated_at: i64,
            data_string: String,
        }

        let row = conn
            .call(move |c| -> tokio_rusqlite::rusqlite::Result<ConfigRow> {
                c.query_one(
                    "SELECT id, version, updated_at, data FROM config WHERE id=?1",
                    params![],
                    |r| {
                        let id: i8 = r.get(0)?;
                        let version: i64 = r.get(1)?;
                        let updated_at: i64 = r.get(2)?;
                        let data_string: String = r.get(3)?;
                        Ok(ConfigRow {
                            id,
                            version,
                            updated_at,
                            data_string,
                        })
                    },
                )
            })
            .await?;

        Ok(Self {
            id: row.id,
            version: row.version,
            updated_at: row.updated_at,
            data: serde_json::to_value(row.data_string)?,
        })
    }

    pub async fn update(&mut self, conn: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = conn.conn().await;

        let data_str = self.data.to_string();

        let version = self.version + 1;
        let updated_at_ts_ms: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        conn.call(move |c| -> rusqlite::Result<()> {
            c.execute(
                "UPDATE config SET version=?1, updated_at=?2, data=?3 WHERE id=1",
                params![version, updated_at_ts_ms, data_str],
            )?;
            Ok(())
        })
        .await?;

        self.version = version;
        self.updated_at = updated_at_ts_ms;

        Ok(())
    }

    pub async fn update_data(conn: &DatabaseConnection, data: &serde_json::Value) -> anyhow::Result<()> {
        let conn = conn.conn().await;
        let data_str = data.to_string();

        let updated_at_ts_ms: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        conn.call(move |c| -> rusqlite::Result<()> {
            c.execute(
                "UPDATE config SET version=version+1, updated_at=?1, data=?2 WHERE id=1",
                params![updated_at_ts_ms, data_str],
            )?;
            Ok(())
        })
        .await?;

        Ok(())
    }
}
