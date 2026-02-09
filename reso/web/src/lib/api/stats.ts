import type { KyInstance } from 'ky';

export class Stats {
	private httpClient: KyInstance;

	public constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async live() {
		const response = await this.httpClient.get('api/stats/live');
		const json = await response.json<LiveStats>();
		return json;
	}
}

export interface LiveStats {
	total: number;
	blocked: number;
	cached: number;
	errors: number;
	sum_duration: number;
	live_since: number;
}
