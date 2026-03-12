import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useBlockDomain() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: (domain: string) => apiClient.blocklist.create(domain),
	});
}
