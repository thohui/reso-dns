import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';
import type { ActivityListRequest } from '@/lib/api/activity';

export const ACTIVITIES_PAGE_SIZE = 50;
export const activitiesQueryKey = (req: ActivityListRequest) => [
	'activities',
	req.top,
	req.skip,
	req.filter,
	req.sort,
	req.dir,
	req.count,
];

export function useActivities(req: ActivityListRequest) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: activitiesQueryKey(req),
		queryFn: async () => apiClient.activities.list(req),
		placeholderData: keepPreviousData,
	});
}
