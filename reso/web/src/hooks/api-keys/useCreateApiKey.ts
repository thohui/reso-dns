import { useApiClient } from '@/contexts/ApiClientContext';
import { useMutation } from '@tanstack/react-query';

export function useCreateApiKey() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: (payload: { display_name: string; expires_at?: number; }) => apiClient.apiKeys.create(payload),
	});
}
