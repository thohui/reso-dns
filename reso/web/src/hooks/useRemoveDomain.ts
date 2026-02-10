import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useRemoveDomain() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: async (domain: string) => apiClient.blocklist.remove(domain),
	});
}
