import type { KyInstance } from 'ky';
import type { PagedRequest, PagedResponse } from './pagination';

export class LocalRecords {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list(req: PagedRequest) {
		const response = await this.httpClient.get(
			`api/local-records?top=${req.top}&skip=${req.skip}`,
		);
		return await response.json<PagedResponse<LocalRecord>>();
	}

	public async create(record: {
		name: string;
		record_type: number;
		value: string;
		ttl?: number;
	}) {
		await this.httpClient.post('api/local-records', {
			json: record,
		});
	}

	public async remove(id: number) {
		await this.httpClient.delete('api/local-records', {
			json: { id },
		});
	}

	public async toggle(id: number) {
		await this.httpClient.patch('api/local-records/toggle', {
			json: { id },
		});
	}
}

export interface LocalRecord {
	id: number;
	name: string;
	record_type: number;
	value: string;
	ttl: number;
	enabled: boolean;
	created_at: number;
}
