-- rebuild to add match_type to the unique constraint
CREATE TABLE local_records_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    match_type TEXT NOT NULL DEFAULT 'exact',
    record_type INTEGER NOT NULL,
    value TEXT NOT NULL,
    ttl INTEGER NOT NULL DEFAULT 300,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    UNIQUE(name, match_type, record_type, value),
    CHECK (enabled IN (0, 1)),
    CHECK (match_type IN ('exact', 'wildcard'))
);

-- migrate existing wildcard rows
INSERT INTO local_records_new (id, name, match_type, record_type, value, ttl, enabled, created_at)
SELECT
    id,
    CASE WHEN name LIKE '*.%' THEN SUBSTR(name, 3) ELSE name END,
    CASE WHEN name LIKE '*.%' THEN 'wildcard' ELSE 'exact' END,
    record_type,
    value,
    ttl,
    enabled,
    created_at
FROM local_records;

DROP TABLE local_records;
ALTER TABLE local_records_new RENAME TO local_records;
