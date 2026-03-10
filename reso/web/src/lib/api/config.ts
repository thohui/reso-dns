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
}

export type Upstream = string;

export interface ForwarderConfig {
	upstreams: string[];
}
