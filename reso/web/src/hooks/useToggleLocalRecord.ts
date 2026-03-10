import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useToggleLocalRecord() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: async (id: number) => apiClient.localRecords.toggle(id),
	});
}
