CREATE TABLE
  IF NOT EXISTS dns_query_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_ms INTEGER NOT NULL,
    transport INTEGER NOT NULL,
    client TEXT,
    qname TEXT NOT NULL,
    qtype INTEGER NOT NULL,
    rcode INTEGER NOT NULL,
    blocked BOOLEAN NOT NULL,
    cache_hit BOOLEAN NOT NULL,
    dur_us INTEGER NOT NULL,
    -- enforce boolean-ness
    CHECK (blocked IN (0, 1)),
    CHECK (cache_hit IN (0, 1))
  );

CREATE INDEX IF NOT EXISTS idx_dns_query_log_ts ON dns_query_log (ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_qname_ts ON dns_query_log (qname, ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_client_ts ON dns_query_log (client, ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_blocked_ts ON dns_query_log (blocked, ts_ms);

CREATE TABLE
  IF NOT EXISTS dns_error_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_ms INTEGER NOT NULL,
    transport INTEGER NOT NULL,
    client TEXT,
    message TEXT NOT NULL,
    type INTEGER NOT NULL
  );

CREATE INDEX IF NOT EXISTS idx_dns_error_log_ts ON dns_error_log (ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_error_log_type ON dns_error_log (type);

CREATE TABLE
  IF NOT EXISTS blocklist (domain TEXT PRIMARY KEY);