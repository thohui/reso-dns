import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export const localRecordsQueryKey = ['local-records'];

export function useLocalRecords() {
	const apiClient = useApiClient();

	return useQuery({
		queryKey: localRecordsQueryKey,
		queryFn: async () => apiClient.localRecords.list({ top: 1000, skip: 0 }),
		placeholderData: keepPreviousData,
	});
}
