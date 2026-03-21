import { useMutation } from '@tanstack/react-query';
import { useApiClient } from '../contexts/ApiClientContext';
import type { ListAction } from '../lib/api/domain-rules';

export function useAddSubscription() {
	const apiClient = useApiClient();

	return useMutation({
		mutationFn: ({
			name,
			url,
			list_type,
			sync_enabled,
		}: {
			name: string;
			url: string;
			list_type: ListAction;
			sync_enabled: boolean;
		}) =>
			apiClient.listSubscriptions.create(name, url, list_type, sync_enabled),
	});
}
