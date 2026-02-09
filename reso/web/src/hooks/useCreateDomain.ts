import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useCreateDomain() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: (domain: string) => apiClient.blocklist.create(domain),
	});
}
