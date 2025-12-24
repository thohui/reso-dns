CREATE TABLE IF NOT EXISTS dns_query_log (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  ts_ms       INTEGER NOT NULL,      -- unix millis
  transport   INTEGER NOT NULL,      -- 0=udp, 1=tcp
  client      TEXT,                 -- e.g. "192.168.1.50" or null
  qname       TEXT NOT NULL,         -- FQDN
  qtype       INTEGER NOT NULL,      -- e.g. 1=A, 16=TXT
  rcode       INTEGER NOT NULL,      -- e.g. 0=NOERROR, 3=NXDOMAIN

  blocked     BOOLEAN NOT NULL,
  cache_hit   BOOLEAN NOT NULL,

  dur_us      INTEGER NOT NULL,


  -- enforce boolean-ness
  CHECK (blocked IN (0, 1)),
  CHECK (cache_hit IN (0, 1))

);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_ts
  ON dns_query_log(ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_qname_ts
  ON dns_query_log(qname, ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_client_ts
  ON dns_query_log(client, ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_blocked_ts
  ON dns_query_log(blocked, ts_ms);


CREATE TABLE IF NOT EXISTS blocklist	 (
	domain TEXT PRIMARY KEY
);