CREATE TABLE clients (
    ip TEXT PRIMARY KEY,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL
);

CREATE INDEX idx_clients_last_seen_at ON clients (last_seen_at);
