CREATE INDEX IF NOT EXISTS idx_dns_query_log_cache_hit_ts ON dns_query_log (cache_hit, ts_ms);

CREATE INDEX IF NOT EXISTS idx_dns_query_log_rate_limited_ts ON dns_query_log (rate_limited, ts_ms);