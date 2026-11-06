import type { KyInstance } from 'ky';
import type { PagedRequest, PagedResponse } from './pagination';

export class Clients {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list(req: PagedRequest) {
		const params = new URLSearchParams({
			top: String(req.top),
			skip: String(req.skip),
		});
		if (req.search) params.set('search', req.search);
		const response = await this.httpClient.get(`api/clients?${params}`);
		return await response.json<PagedResponse<Client>>();
	}

	public async update(ip: string, payload: { display_name: string | null }) {
		await this.httpClient.put(`api/clients/${encodeURIComponent(ip)}`, {
			json: payload,
		});
	}
}

export interface Client {
	ip: string;
	first_seen_at: number;
	last_seen_at: number;
	hostname: string | null;
	hostname_checked_at: number | null;
	display_name: string | null;
}
