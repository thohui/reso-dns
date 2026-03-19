import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useRemoveDomainRule() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: async (domain: string) => apiClient.domainRules.remove(domain),
	});
}
