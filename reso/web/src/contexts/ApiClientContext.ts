import { createContext, useContext } from 'react';
import type { ApiClient } from '../lib/api/client';

export const ApiClientContext = createContext<ApiClient | null>(null);

export function useApiClient() {
	const ctx = useContext(ApiClientContext);

	if (!ctx) {
		throw new Error('ApiClientContext is not initialized!');
	}

	return ctx;
}
