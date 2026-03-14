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

	public async top(range: TopRange) {
		const response = await this.httpClient.get('api/stats/top', {
			searchParams: { range },
		});
		return response.json<TopResponse>();
	}

	public async timeline(range: TopRange) {
		const response = await this.httpClient.get('api/stats/timeline', {
			searchParams: { range },
		});
		return response.json<TimelineResponse>();
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

export type TopRange =
	| '5min'
	| 'hour'
	| 'day'
	| 'week'
	| 'month'
	| 'year'
	| 'all';

export interface TopEntry {
	name: string;
	count: number;
}

export interface TopResponse {
	clients: TopEntry[];
	domains: TopEntry[];
	blocked_domains: TopEntry[];
}

export interface TimelineBucket {
	ts: number;
	total: number;
	blocked: number;
	cached: number;
	errors: number;
	sum_duration: number;
}

export interface TimelineResponse {
	buckets: TimelineBucket[];
}
