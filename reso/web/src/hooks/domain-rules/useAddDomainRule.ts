import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';
import type { ListAction } from '@/lib/api/domain-rules';

export function useAddDomainRule() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: ({ domain, action }: { domain: string; action: ListAction }) =>
			apiClient.domainRules.create(domain, action),
	});
}
