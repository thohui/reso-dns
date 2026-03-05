ALTER TABLE dns_query_log ADD COLUMN rate_limited INTEGER NOT NULL DEFAULT 0;

DROP VIEW IF EXISTS activity_log;

CREATE VIEW
	activity_log AS
SELECT
	ts_ms,
	'query' AS kind,
	id AS source_id,
	transport,
	client,
	qname,
	qtype,
	rcode,
	blocked,
	cache_hit,
	dur_ms,
	NULL AS error_type,
	NULL AS error_message,
	rate_limited
FROM
	dns_query_log
UNION ALL
SELECT
	ts_ms,
	'error' AS kind,
	id AS source_id,
	transport,
	client,
	qname,
	qtype,
	NULL AS rcode,
	NULL AS blocked,
	NULL AS cache_hit,
	dur_ms,
	type AS error_type,
	message AS error_message,
	NULL AS rate_limited
FROM
	dns_error_log;
