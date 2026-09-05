ALTER TABLE activity_log ADD COLUMN upstream_protocol INTEGER;
ALTER TABLE activity_log ADD COLUMN rule_id BLOB;

CREATE INDEX IF NOT EXISTS idx_activity_log_rule_id_ts ON activity_log (rule_id, ts_ms);
