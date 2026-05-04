import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useRemoveSubscription() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: (id: string) => apiClient.listSubscriptions.remove(id),
	});
}
