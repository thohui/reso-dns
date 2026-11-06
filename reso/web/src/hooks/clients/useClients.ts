import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export const CLIENTS_PAGE_SIZE = 25;

export function useClients(page: number, search: string) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: ['clients', page, search],
		queryFn: () =>
			apiClient.clients.list({
				top: CLIENTS_PAGE_SIZE,
				skip: page * CLIENTS_PAGE_SIZE,
				search: search || undefined,
			}),
		placeholderData: keepPreviousData,
	});
}
