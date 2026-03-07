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
		if (f.blocked) params.set('blocked', f.blocked.toString());
		if (f.cache_hit) params.set('cache_hit', f.cache_hit.toString());
		if (f.rate_limited) params.set('rate_limited', f.rate_limited.toString());

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

export const RECORD_TYPES: Record<number, string> = {
	1: 'A',
	2: 'NS',
	3: 'MD',
	4: 'MF',
	5: 'CNAME',
	6: 'SOA',
	7: 'MB',
	8: 'MG',
	9: 'MR',
	10: 'NULL',
	11: 'WKS',
	12: 'PTR',
	13: 'HINFO',
	14: 'MINFO',
	15: 'MX',
	16: 'TXT',
	17: 'RP',
	18: 'AFSDB',
	19: 'X25',
	20: 'ISDN',
	21: 'RT',
	22: 'NSAP',
	23: 'NSAPPTR',
	24: 'SIG',
	25: 'KEY',
	26: 'PX',
	27: 'GPOS',
	28: 'AAAA',
	29: 'LOC',
	30: 'NXT',
	31: 'EID',
	32: 'NIMLOC',
	33: 'SRV',
	34: 'ATMA',
	35: 'NAPTR',
	36: 'KX',
	37: 'CERT',
	38: 'A6',
	39: 'DNAME',
	40: 'SINK',
	41: 'OPT',
	42: 'APL',
	43: 'DS',
	44: 'SSHFP',
	45: 'IPSECKEY',
	46: 'RRSIG',
	47: 'NSEC',
	48: 'DNSKEY',
	49: 'DHCID',
	50: 'NSEC3',
	51: 'NSEC3PARAM',
	52: 'TLSA',
	53: 'SMIMEA',
	55: 'HIP',
	56: 'NINFO',
	57: 'RKEY',
	58: 'TALINK',
	59: 'CDS',
	60: 'CDNSKEY',
	61: 'OPENPGPKEY',
	62: 'CSYNC',
	63: 'ZONEMD',
	64: 'SVCB',
	65: 'HTTPS',
	66: 'DSYNC',
	67: 'HHIT',
	68: 'BRID',
	99: 'SPF',
	100: 'UINFO',
	101: 'UID',
	102: 'GID',
	103: 'UNSPEC',
	104: 'NID',
	105: 'L32',
	106: 'L64',
	107: 'LP',
	108: 'EUI48',
	109: 'EUI64',
	128: 'NXNAME',
	249: 'TKEY',
	250: 'TSIG',
	251: 'IXFR',
	252: 'AXFR',
	253: 'MAILB',
	254: 'MAILA',
	255: 'ANY',
	256: 'URI',
	257: 'CAA',
	258: 'AVC',
	259: 'DOA',
	260: 'AMTRELAY',
	261: 'RESINFO',
	262: 'WALLET',
	263: 'CLA',
	264: 'IPN',
};

export function getRecordType(id: number) {
	return RECORD_TYPES[id] ?? id.toString();
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
