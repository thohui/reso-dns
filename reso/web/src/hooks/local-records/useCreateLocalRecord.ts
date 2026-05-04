import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useCreateLocalRecord() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: (record: {
			name: string;
			record_type: number;
			value: string;
			ttl?: number;
		}) => apiClient.localRecords.create(record),
	});
}
