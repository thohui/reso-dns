import type { KyInstance } from 'ky';
import type { PagedRequest, PagedResponse } from './pagination';

export type ListAction = 'block' | 'allow';

export class DomainRules {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list(req: PagedRequest & { search?: string }) {
		const params = new URLSearchParams({
			top: String(req.top),
			skip: String(req.skip),
		});

		if (req.search) {
			params.set('search', req.search);
		}

		const response = await this.httpClient.get(`api/domain-rules?${params}`);

		return await response.json<PagedResponse<DomainRule>>();
	}

	public async remove(domain: string) {
		await this.httpClient.delete('api/domain-rules', { json: { domain } });
	}

	public async create(domain: string, action: ListAction = 'block') {
		await this.httpClient.post('api/domain-rules', {
			json: { domain, action },
		});
	}

	public async toggle(domain: string) {
		await this.httpClient.patch('api/domain-rules/toggle', {
			json: { domain },
		});
	}

	public async update(domain: string, action: ListAction) {
		await this.httpClient.put('api/domain-rules', { json: { domain, action } });
	}
}

export interface DomainRule {
	id: string;
	domain: string;
	action: ListAction;
	created_at: number;
	enabled: boolean;
	subscription_id: string | null;
}
