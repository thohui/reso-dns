UPDATE config_settings
SET key = 'logs.truncation_enabled'
WHERE key = 'logs.enabled';
