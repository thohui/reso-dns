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

CREATE VIEW
	activity_log AS
SELECT
	ts_ms,
	'query' AS kind,
	id AS source_id,
	transport,
	client,
	qname,
	qtype,
	rcode,
	blocked,
	cache_hit,
	dur_ms,
	NULL AS error_type,
	NULL AS error_message
FROM
	dns_query_log
UNION ALL
SELECT
	ts_ms,
	'error' AS kind,
	id AS source_id,
	transport,
	client,
	qname,
	qtype,
	NULL AS rcode,
	NULL AS blocked,
	NULL AS cache_hit,
	dur_ms,
	type AS error_type,
	message AS error_message
FROM
	dns_error_log;

CREATE TABLE
	IF NOT EXISTS config (
		id INTEGER PRIMARY KEY CHECK (id = 1),
		version INTEGER NOT NULL,
		updated_at INTEGER NOT NULL,
		data TEXT NOT NULL,
		CHECK (json_valid (data))
	);

INSERT
OR IGNORE INTO config (id, version, updated_at, data)
VALUES
	(
		1,
		1,
		(CAST(strftime ('%s', 'now') AS INTEGER) * 1000),
		'{}'
	);