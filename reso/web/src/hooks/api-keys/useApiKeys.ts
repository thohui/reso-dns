import { useApiClient } from '@/contexts/ApiClientContext';
import { keepPreviousData, useQuery } from '@tanstack/react-query';

export const API_KEYS_PAGE_SIZE = 25;
export const apiKeysQueryKey = (page: number, search: string) => [
	'api-keys',
	page,
	search,
];

export function useApiKeys(page: number, search: string) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: apiKeysQueryKey(page, search),
		queryFn: () =>
			apiClient.apiKeys.list({
				top: API_KEYS_PAGE_SIZE,
				skip: page * API_KEYS_PAGE_SIZE,
				search: search || undefined,
			}),
		placeholderData: keepPreviousData,
	});
}
