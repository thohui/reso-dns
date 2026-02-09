import type { KyInstance } from 'ky';
import type { PagedRequest, PagedResponse } from './pagination';

export class Blocklist {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list(req: PagedRequest) {
		const response = await this.httpClient.get(
			`api/blocklist?top=${req.top}&skip=${req.skip}`,
		);
		return await response.json<PagedResponse<BlockedDomain>>();
	}

	public async remove(domain: string) {
		await this.httpClient.delete('api/blocklist', {
			json: {
				domain,
			},
		});
	}

	public async create(domain: string) {
		await this.httpClient.post('api/blocklist', {
			json: {
				domain: domain,
			},
		});
	}
}

export interface BlockedDomain {
	domain: string;
	created_at: number;
}
