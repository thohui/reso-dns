import { useSuspenseQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export const useConfigQueryKey = ['config'];

export function useConfig() {
	const apiClient = useApiClient();
	return useSuspenseQuery({
		queryKey: useConfigQueryKey,
		queryFn: () => apiClient.config.get(),
	});
}
