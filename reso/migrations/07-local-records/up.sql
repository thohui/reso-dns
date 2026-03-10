CREATE TABLE IF NOT EXISTS local_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    record_type INTEGER NOT NULL,
    value TEXT NOT NULL,
    ttl INTEGER NOT NULL DEFAULT 300,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    UNIQUE(name, record_type, value),
    CHECK (enabled IN (0, 1))
);
