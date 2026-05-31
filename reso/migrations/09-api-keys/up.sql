CREATE TABLE
	IF NOT EXISTS api_keys (
		id BLOB PRIMARY KEY,
		display_name TEXT NOT NULL,
		user_id BLOB NOT NULL,
		key_hash TEXT NOT NULL UNIQUE,
		created_at INTEGER NOT NULL,
		expires_at INTEGER,
		FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
	);