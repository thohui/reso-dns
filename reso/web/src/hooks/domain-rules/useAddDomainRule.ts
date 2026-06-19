import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';
import type { ListAction, MatchType } from '@/lib/api/domain-rules';

export function useAddDomainRule() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: ({
			domain,
			matchType,
			action,
		}: {
			domain: string;
			matchType: MatchType;
			action: ListAction;
		}) => apiClient.domainRules.create(domain, matchType, action),
	});
}
