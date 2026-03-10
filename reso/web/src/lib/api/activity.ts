import type { KyInstance } from 'ky';
import type { PagedResponse } from './pagination';

export interface ActivityListFilter {
	client?: string;
	qname?: string;
	qtype?: number;
	blocked?: boolean;
	cache_hit?: boolean;
	rate_limited?: boolean;
	error_only?: boolean;
}

export type SortColumn = 'timestamp' | 'client' | 'qname' | 'duration';
export type SortDir = 'asc' | 'desc';

export interface ActivityListRequest {
	top: number;
	skip: number;
	filter?: ActivityListFilter;
	sort?: SortColumn;
	dir?: SortDir;
}

export class Activities {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list(req: ActivityListRequest) {
		const params = new URLSearchParams();
		params.set('top', req.top.toString());
		params.set('skip', req.skip.toString());

		const f = req.filter ?? {};
		if (f.client) params.set('client', f.client);
		if (f.qname) params.set('qname', f.qname);
		if (f.qtype != null) params.set('qtype', f.qtype.toString());
		if (f.blocked != null) params.set('blocked', f.blocked.toString());
		if (f.cache_hit != null) params.set('cache_hit', f.cache_hit.toString());
		if (f.rate_limited != null)
			params.set('rate_limited', f.rate_limited.toString());

		if (f.error_only) params.set('error_only', 'true');
		if (req.sort) params.set('sort', req.sort);
		if (req.dir) params.set('dir', req.dir);

		const response = await this.httpClient.get(`api/activity?${params}`);
		return response.json<PagedResponse<Activity>>();
	}
}

export type Activity = QueryActivity | ErrorActivity;

export interface ActivityBase {
	timestamp: number;
	transport: number;
	client: string | null;
	duration: number;
	qname: string | null;
	qtype: number | null;
}

export interface QueryActivity extends ActivityBase {
	kind: 'query';
	d: {
		source_id: number;
		rcode: number;
		blocked: boolean;
		cache_hit: boolean;
		rate_limited: boolean;
	};
}

export interface ErrorActivity extends ActivityBase {
	kind: 'error';
	d: {
		source_id: number;
		error_type: number;
		message: string;
	};
}

export const TRANSPORT_LABELS: Record<number, string> = {
	0: 'UDP',
	1: 'TCP',
	2: 'DoT',
	3: 'DoH',
	4: 'DoQ',
};

export function getTransportLabel(id: number) {
	return TRANSPORT_LABELS[id] ?? 'Unknown';
}

export const RCODE_LABELS: Record<number, string> = {
	0: 'NOERROR',
	1: 'FORMERR',
	2: 'SERVFAIL',
	3: 'NXDOMAIN',
	4: 'NOTIMP',
	5: 'REFUSED',
};

export function getResponseCodeLabel(id: number) {
	return RCODE_LABELS[id] ?? id.toString();
}

export const ERROR_TYPE_LABELS: Record<number, string> = {
	0: 'Timeout',
	1: 'Connection Refused',
	2: 'Network Unreachable',
	3: 'Invalid Response',
	4: 'Server Failure',
	5: 'Internal Error',
};

export function getErrorTypeLabel(id: number) {
	return ERROR_TYPE_LABELS[id] ?? 'Unknown';
}
