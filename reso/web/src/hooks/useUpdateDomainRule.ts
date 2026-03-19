import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';
import type { ListAction } from '../lib/api/domain-rules';

export function useUpdateDomainRule() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: ({ domain, action }: { domain: string; action: ListAction }) =>
			apiClient.domainRules.update(domain, action),
	});
}
