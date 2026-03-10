import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useRemoveLocalRecord() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: async (id: number) => apiClient.localRecords.remove(id),
	});
}
