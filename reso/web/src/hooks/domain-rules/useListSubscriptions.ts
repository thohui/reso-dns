import { useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export const listSubscriptionsQueryKey = ['list-subscriptions'];

export function useListSubscriptions() {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: listSubscriptionsQueryKey,
		queryFn: () => apiClient.listSubscriptions.list(),
	});
}
