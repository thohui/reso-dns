import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useToggleDomainRule() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: async (domain: string) => apiClient.domainRules.toggle(domain),
	});
}
