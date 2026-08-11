UPDATE config_settings
SET key = 'logs.truncation_enabled'
WHERE key = 'logs.enabled';

-- Convert dns.forwarder.upstreams config setting to the new tagged union shape.
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
						CASE WHEN instr(spec, ':') > 0 THEN spec ELSE spec || ':53' END
					)
				)
			FROM
				(
					SELECT trim(je.value) AS spec
					FROM json_each(config_settings.value) AS je
				)
			WHERE
				spec GLOB '[0-9]*.[0-9]*.[0-9]*.[0-9]*'
		),
		'[]'
	)
WHERE
	key = 'dns.forwarder.upstreams';
