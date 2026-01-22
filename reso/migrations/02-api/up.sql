CREATE TABLE
	IF NOT EXISTS users (
		id BLOB PRIMARY KEY,
		password_hash TEXT NOT NULL,
		name TEXT NOT NULL UNIQUE,
		permissions INTEGER DEFAULT 0,
		created_at INTEGER NOT NULL -- ms
	);

CREATE TABLE
	IF NOT EXISTS user_sessions (
		id BLOB PRIMARY KEY,
		user_id BLOB NOT NULL,
		created_at INTEGER NOT NULL,
		expires_at INTEGER NOT NULL,
		FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
	);