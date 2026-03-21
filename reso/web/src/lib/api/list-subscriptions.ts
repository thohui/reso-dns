import type { KyInstance } from 'ky';
import type { ListAction } from './domain-rules';

export class ListSubscriptions {
	private httpClient: KyInstance;

	constructor(httpClient: KyInstance) {
		this.httpClient = httpClient;
	}

	public async list() {
		return await this.httpClient
			.get('api/list-subscriptions')
			.json<ListSubscription[]>();
	}

	public async create(
		name: string,
		url: string,
		list_type: ListAction,
		sync_enabled: boolean,
	) {
		await this.httpClient.post('api/list-subscriptions', {
			json: { name, url, list_type, sync_enabled },
		});
	}

	public async remove(id: string) {
		await this.httpClient.delete('api/list-subscriptions', { json: { id } });
	}

	public async toggle(id: string) {
		await this.httpClient.patch('api/list-subscriptions/toggle', {
			json: { id },
		});
	}

	public async toggleSync(id: string) {
		await this.httpClient.patch('api/list-subscriptions/toggle-sync', {
			json: { id },
		});
	}
}

export interface ListSubscription {
	id: string;
	name: string;
	url: string;
	list_type: ListAction;
	enabled: boolean;
	created_at: number;
	last_synced_at: number | null;
	domain_count: number;
	sync_enabled: boolean;
}
