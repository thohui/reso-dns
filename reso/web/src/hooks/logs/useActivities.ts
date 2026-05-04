import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';
import type { ActivityListRequest } from '@/lib/api/activity';

export function useActivities(req: ActivityListRequest) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: ['activities', req],
		queryFn: async () => {
			return apiClient.activities.list(req);
		},
		placeholderData: keepPreviousData,
	});
}
