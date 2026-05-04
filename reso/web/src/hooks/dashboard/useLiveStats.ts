import { useApiClient } from '@/contexts/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export function useLiveStats() {
	const apiClient = useApiClient();
	return useQuery({
		queryKey: ['live-stats'],
		queryFn: async () => apiClient.stats.live(),
		// we only fetch this for the uptime, so we can get away with a long stale time.
		staleTime: 1000 * 60 * 60,
	});
}
