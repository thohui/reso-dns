import type { ConfigModel } from '@/lib/api/config';
import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';

export function useUpdateConfig() {
	const apiClient = useApiClient();
	return useMutation({
		mutationFn: (config: ConfigModel) => apiClient.config.update(config),
	});
}
