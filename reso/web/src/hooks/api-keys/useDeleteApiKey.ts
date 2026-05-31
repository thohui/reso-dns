import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useDeleteApiKey() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: (id: string) => apiClient.apiKeys.remove(id),
	});
}
