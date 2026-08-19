UPDATE config_settings
SET key = 'logs.truncation_enabled'
WHERE key = 'logs.enabled';

-- Convert dns.forwarder.upstreams config setting to the new tagged union shape.
-- Hostnames are dropped, the resolver only accepts IP endpoints.
UPDATE config_settings
SET
	value = COALESCE(
		(
			SELECT
				json_group_array(
					json_object(
						'kind',
						'plain',
						'endpoint',
						CASE
							-- [IPv6], the closing bracket means the colons are part of the address.
							WHEN spec GLOB '[[]*]' THEN spec || ':53'
							-- [IPv6]:port
							WHEN spec GLOB '[[]*]:[0-9]*' THEN spec
							-- IPv4:port
							WHEN instr(spec, ':') > 0 THEN spec
							ELSE spec || ':53'
						END
					)
				)
			FROM
				(
					SELECT trim(je.value) AS spec
					FROM json_each(config_settings.value) AS je
				)
			WHERE
				spec GLOB '[0-9]*.[0-9]*.[0-9]*.[0-9]*'
				OR spec GLOB '[[]*]'
				OR spec GLOB '[[]*]:[0-9]*'
		),
		'[]'
	)
WHERE
	key = 'dns.forwarder.upstreams'
	AND json_valid(value);
