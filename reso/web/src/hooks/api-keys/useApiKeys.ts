import { useApiClient } from '@/contexts/ApiClientContext';
import { useSuspenseQuery } from '@tanstack/react-query';

export const apiKeysQueryKey = ['api-keys'];

export function useApiKeys() {
	const apiClient = useApiClient();

	// todo: add pagination support.
	return useSuspenseQuery({
		queryKey: apiKeysQueryKey,
		queryFn: async () => apiClient.apiKeys.list({ top: 100, skip: 0 }),
	});
}
