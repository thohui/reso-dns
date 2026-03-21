CREATE TABLE list_subscriptions (
    id             BLOB    PRIMARY KEY NOT NULL,
    name           TEXT    NOT NULL,
    url            TEXT    NOT NULL UNIQUE,
    list_type      TEXT    NOT NULL DEFAULT 'block' CHECK (list_type IN ('block', 'allow')),
    enabled        INTEGER NOT NULL DEFAULT 1,
    last_synced_at INTEGER,
    domain_count   INTEGER NOT NULL DEFAULT 0,
    hash           TEXT,
    created_at     INTEGER NOT NULL,
    sync_enabled   INTEGER NOT NULL DEFAULT 1,
    CHECK (enabled IN (0, 1))
    CHECK (sync_enabled IN (0, 1))
);

DROP TABLE blocklist;

CREATE TABLE domain_rules (
    id              BLOB    PRIMARY KEY NOT NULL,
    domain          TEXT    NOT NULL UNIQUE,
    action          TEXT    NOT NULL DEFAULT 'block' CHECK (action IN ('block', 'allow')),
    created_at      INTEGER NOT NULL,
    enabled         INTEGER NOT NULL DEFAULT 1,
    subscription_id BLOB    REFERENCES list_subscriptions(id) ON DELETE CASCADE,
    CHECK (enabled IN (0, 1))
);

