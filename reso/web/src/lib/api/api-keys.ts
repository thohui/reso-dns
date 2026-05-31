import type { KyInstance } from 'ky';
import type { PagedRequest, PagedResponse } from './pagination';

export class ApiKeys {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list(req: PagedRequest) {
		const response = await this.httpClient.get(
			`api/api-keys?top=${req.top}&skip=${req.skip}`,
		);
		return await response.json<PagedResponse<ApiKey>>();
	}


	public async create(payload: { display_name: string; expires_at?: number; }) {
		const response = await this.httpClient.post('api/api-keys', {
			json: payload,
		});
		return await response.json<CreatedApiKey>();
	}

	public async remove(id: string) {
		await this.httpClient.delete(`api/api-keys/${id}`);
	}

}

export interface ApiKey {
	id: string;
	display_name: string;
	created_by: string;
	created_at: number;
	expires_at: number | null;
}

export interface CreatedApiKey extends ApiKey {
	token: string;
}
