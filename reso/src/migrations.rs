use reso_database::Migration;

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 0,
    sql: r#"
				CREATE TABLE IF NOT EXISTS blocklist (
						domain TEXT PRIMARY KEY
				);
				"#,
}];
