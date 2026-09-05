ALTER TABLE activity_log ADD COLUMN upstream_protocol INTEGER;
ALTER TABLE activity_log ADD COLUMN rule_id BLOB;

CREATE INDEX idx_activity_log_rule_id_ts ON activity_log (rule_id, ts_ms) WHERE rule_id IS NOT NULL;
