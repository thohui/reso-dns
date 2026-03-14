CREATE TABLE
    metrics_by_client (
        bucket_ts INTEGER NOT NULL,
        client TEXT NOT NULL,
        total_count INTEGER NOT NULL DEFAULT 0,
        blocked_count INTEGER NOT NULL DEFAULT 0,
        cached_count INTEGER NOT NULL DEFAULT 0,
        error_count INTEGER NOT NULL DEFAULT 0,
        sum_duration INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (bucket_ts, client)
    );

CREATE TABLE
    metrics_by_domain (
        bucket_ts INTEGER NOT NULL,
        qname TEXT NOT NULL,
        total_count INTEGER NOT NULL DEFAULT 0,
        blocked_count INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (bucket_ts, qname)
    );

CREATE INDEX idx_client_metrics_client_ts_total ON metrics_by_client (client, bucket_ts, total_count);
CREATE INDEX idx_domain_metrics_qname_ts_total ON metrics_by_domain (qname, bucket_ts, total_count);
CREATE INDEX idx_domain_metrics_qname_ts_blocked ON metrics_by_domain (qname, bucket_ts, blocked_count);

