ALTER TABLE domain_rules ADD COLUMN match_type TEXT NOT NULL DEFAULT 'domain'
    CHECK (match_type IN ('exact', 'wildcard', 'domain'));

-- migrate existing wildcard rows
UPDATE domain_rules
    SET match_type = 'wildcard', domain = SUBSTR(domain, 3)
    WHERE domain LIKE '*.%';

ALTER TABLE list_subscriptions DROP COLUMN list_type;
