import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export const DOMAIN_RULES_PAGE_SIZE = 50;

export function useDomainRules(page: number, search: string) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: ['domain-rules', page, search],
		queryFn: () =>
			apiClient.domainRules.list({
				top: DOMAIN_RULES_PAGE_SIZE,
				skip: page * DOMAIN_RULES_PAGE_SIZE,
				search: search || undefined,
			}),
		placeholderData: keepPreviousData,
	});
}
