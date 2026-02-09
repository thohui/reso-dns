import ky, { type KyInstance } from 'ky';
import { Activities } from './activity';
import { Blocklist } from './blocklist';
import { Stats } from './stats';

interface ApiError {
	error?: string;
	message?: string;
}

export class ApiClient {
	private eventBus: EventBus;
	private httpClient: KyInstance;
	private authenticated: boolean;

	public activities: Activities;
	public stats: Stats;
	public blocklist: Blocklist;

	constructor() {
		this.eventBus = new EventBus();
		this.httpClient = ky.create({
			prefixUrl: import.meta.env.DEV ? 'http://localhost:8080' : undefined,
			credentials: 'include',
			hooks: {
				afterResponse: [
					async (_, __, resp) => {
						if (resp.status === 401) {
							const errorMessage = await resp.json<ApiError>();
							if (errorMessage?.error === 'authentication_required') {
								this.setAuthenticated(false);
							}
						}
					},
				],
			},
		});
		this.authenticated = false;

		this.activities = new Activities(this.httpClient);
		this.stats = new Stats(this.httpClient);
		this.blocklist = new Blocklist(this.httpClient);
	}

	public async initialize() {
		try {
			await this.httpClient.post('api/auth/check');
			this.setAuthenticated(true);
		} catch (e) {
			console.log(e);
			this.setAuthenticated(false);
		}
	}

	private async setAuthenticated(value: boolean) {
		this.authenticated = value;
		this.eventBus.emit('auth-change', value);
	}

	public isAuthenticated() {
		return this.authenticated;
	}

	public async login(username: string, password: string) {
		await this.httpClient.post('api/auth/login', {
			json: {
				username: username,
				password: password,
			},
		});
		this.setAuthenticated(true);
	}
	public addEventListener<T extends EventType>(
		event: T,
		listener: EventListener<T>,
	) {
		this.eventBus.on(event, listener);
	}

	public removeEventListener<T extends EventType>(
		event: T,
		listener: EventListener<T>,
	) {
		this.eventBus.off(event, listener);
	}
}

export interface Events {
	'auth-change': boolean;
}

export type EventType = keyof Events;
export type EventListener<T extends EventType> = (payload: Events[T]) => void;

export class EventBus {
	private listeners: Map<EventType, EventListener<any>[]> = new Map();

	public on<T extends EventType>(event: T, listener: EventListener<T>) {
		if (!this.listeners.has(event)) {
			this.listeners.set(event, []);
		}
		this.listeners.get(event)?.push(listener);
	}

	public off<T extends EventType>(event: T, listener: EventListener<T>) {
		if (this.listeners.has(event)) {
			const listeners = this.listeners
				.get(event)!
				.filter((l) => l !== listener);
			this.listeners.set(event, listeners);
		}
	}

	public emit<T extends EventType>(event: T, payload: Events[T]) {
		if (!this.listeners.has(event)) return;
		for (const listener of this.listeners.get(event)!) {
			listener(payload);
		}
	}
}
