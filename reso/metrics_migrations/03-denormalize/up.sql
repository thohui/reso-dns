DROP VIEW IF EXISTS activity_log;

CREATE TABLE
    activity_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts_ms INTEGER NOT NULL,
        kind TEXT NOT NULL, -- 'query' or 'error'
        transport INTEGER NOT NULL,
        client TEXT,
        qname TEXT,
        qtype INTEGER,
        dur_ms INTEGER NOT NULL,
        -- query-specific (NULL for errors)
        rcode INTEGER,
        blocked INTEGER,
        cache_hit INTEGER,
        rate_limited INTEGER,
        -- error-specific (NULL for queries)
        error_type INTEGER,
        error_message TEXT
    );

INSERT INTO
    activity_log (
        ts_ms,
        kind,
        transport,
        client,
        qname,
        qtype,
        dur_ms,
        rcode,
        blocked,
        cache_hit,
        rate_limited
    )
SELECT
    ts_ms,
    'query',
    transport,
    client,
    qname,
    qtype,
    dur_ms,
    rcode,
    blocked,
    cache_hit,
    rate_limited
FROM
    dns_query_log;

INSERT INTO
    activity_log (
        ts_ms,
        kind,
        transport,
        client,
        qname,
        qtype,
        dur_ms,
        error_type,
        error_message
    )
SELECT
    ts_ms,
    'error',
    transport,
    client,
    qname,
    qtype,
    dur_ms,
    type,
    message
FROM
    dns_error_log;

DROP TABLE dns_query_log;

DROP TABLE dns_error_log;

CREATE INDEX idx_activity_log_ts ON activity_log (ts_ms);

CREATE INDEX idx_activity_log_kind_ts ON activity_log (kind, ts_ms);

CREATE INDEX idx_activity_log_client_ts ON activity_log (client, ts_ms);

CREATE INDEX idx_activity_log_qname_ts ON activity_log (qname, ts_ms);

CREATE INDEX idx_activity_log_blocked_ts ON activity_log (blocked, ts_ms);

CREATE INDEX idx_activity_log_cache_hit_ts ON activity_log (cache_hit, ts_ms);

CREATE INDEX idx_activity_log_rate_limited_ts ON activity_log (rate_limited, ts_ms);