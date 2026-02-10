import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '..//contexts/ApiClientContext';

export function useLogout() {
	const apiClient = useApiClient();
	return useMutation({ mutationFn: () => apiClient.logout() });
}
