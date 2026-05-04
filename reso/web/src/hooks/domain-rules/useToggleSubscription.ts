import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useToggleSubscription() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: (id: string) => apiClient.listSubscriptions.toggle(id),
	});
}
