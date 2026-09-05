import { skipToken, useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export const domainRuleDetailsQueryKey = (id: string | null) => [
	'domain-rule-details',
	id,
];

export function useDomainRuleDetails(id: string | null) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: domainRuleDetailsQueryKey(id),
		queryFn: id === null ? skipToken : () => apiClient.domainRules.details(id),
	});
}
