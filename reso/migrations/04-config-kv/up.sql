CREATE TABLE
	IF NOT EXISTS config_settings (
		key TEXT PRIMARY KEY NOT NULL,
		value TEXT NOT NULL,
		updated_at INTEGER NOT NULL
	);

INSERT
OR IGNORE INTO config_settings (key, value, updated_at)
SELECT
	'dns.timeout',
	json_extract (data, '$.dns.timeout'),
	updated_at
FROM
	config
WHERE
	id = 1
	AND json_extract (data, '$.dns.timeout') IS NOT NULL;

INSERT
OR IGNORE INTO config_settings (key, value, updated_at)
SELECT
	'dns.active',
	json_extract (data, '$.dns.active'),
	updated_at
FROM
	config
WHERE
	id = 1
	AND json_extract (data, '$.dns.active') IS NOT NULL;

INSERT
OR IGNORE INTO config_settings (key, value, updated_at)
SELECT
	'dns.forwarder.upstreams',
	json_extract (data, '$.dns.forwarder.upstreams'),
	updated_at
FROM
	config
WHERE
	id = 1
	AND json_extract (data, '$.dns.forwarder.upstreams') IS NOT NULL;

DROP TABLE config;
