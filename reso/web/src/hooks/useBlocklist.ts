import { useQuery } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useBlocklist() {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: ['blocklist'],
		queryFn: async () => apiClient.blocklist.list({ top: 1000, skip: 0 }),
	});
}
