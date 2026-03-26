import { useCallback, useSyncExternalStore } from 'react';
import { useApiClient } from '@/contexts/ApiClientContext';

export function useIsSetupRequired() {
	const apiClient = useApiClient();

	const cb = useCallback(
		(cb: () => void) => {
			apiClient.addEventListener('setup-change', cb);

			return () => {
				apiClient.removeEventListener('setup-change', cb);
			};
		},
		[apiClient],
	);

	const getSnapshot = useCallback(() => {
		return apiClient.isSetupRequired();
	}, [apiClient]);

	return useSyncExternalStore(cb, getSnapshot);
}
