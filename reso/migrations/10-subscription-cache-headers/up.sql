ALTER TABLE list_subscriptions DROP COLUMN hash;
ALTER TABLE list_subscriptions ADD COLUMN etag TEXT;
ALTER TABLE list_subscriptions ADD COLUMN last_modified TEXT;
CREATE INDEX IF NOT EXISTS idx_domain_rules_subscription_id ON domain_rules (subscription_id);
