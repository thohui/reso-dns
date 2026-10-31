import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';
import type { LocalRecordMatchType } from '@/lib/api/local-records';

export function useCreateLocalRecord() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: (record: {
			name: string;
			match_type: LocalRecordMatchType;
			record_type: number;
			value: string;
			ttl?: number;
		}) => apiClient.localRecords.create(record),
	});
}
