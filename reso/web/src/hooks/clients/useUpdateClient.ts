import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useUpdateClient() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: ({
			ip,
			display_name,
		}: {
			ip: string;
			display_name: string | null;
		}) => apiClient.clients.update(ip, { display_name }),
	});
}
