import { useQuery } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useRecentActivity() {
	const apiClient = useApiClient();

	const { data } = useQuery({
		queryKey: ['recent-activity'],
		queryFn: async () => {
			return apiClient.activities.list({ top: 10, skip: 0 });
		},
		refetchInterval: 5 * 1000,
	});

	return data;
}
