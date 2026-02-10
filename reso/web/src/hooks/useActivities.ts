import { useApiClient } from '@/contexts/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export function useActivities(top: number, skip: number) {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: ['activities', top, skip],
		queryFn: async () => {
			return apiClient.activities.list({ top: top, skip: skip });
		}
	});

}