import { useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useListSubscriptions() {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: ['list-subscriptions'],
		queryFn: () => apiClient.listSubscriptions.list(),
	});
}
