import type { KyInstance } from 'ky';

export class Config {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async get() {
		const response = await this.httpClient.get('api/config');
		const json = await response.json<ConfigModel>();
		return json;
	}

	public async update(config: ConfigModel) {
		const response = await this.httpClient.put('api/config', { json: config });
		const json = await response.json<ConfigModel>();
		return json;
	}
}

export interface ConfigModel {
	dns: DnsConfig;
	logs: LogsConfig;
}

export interface LogsConfig {
	enabled: boolean;
	retention_secs: number;
	truncate_interval_secs: number;
}

export type ActiveResolver = 'forwarder';

export interface RateLimitConfig {
	enabled: boolean;
	window_duration: number;
	max_queries_per_window: number;
}

export interface DnsConfig {
	timeout: number;
	active: ActiveResolver;
	forwarder: ForwarderConfig;
	rate_limit: RateLimitConfig;
	security: SecurityConfig;
}

export interface SecurityConfig {
	block_icloud_private_relay: boolean;
	block_firefox_canary: boolean;
	block_designated_resolver: boolean;
}

export type Upstream = string;

export interface ForwarderConfig {
	upstreams: string[];
}
