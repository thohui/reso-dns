import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useToggleDomain() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: async (domain: string) => apiClient.blocklist.toggle(domain),
	});
}
