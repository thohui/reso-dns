import { useCallback, useSyncExternalStore } from 'react';
import { useApiClient } from '../contexts/ApiClientContext';

export function useIsAuthenticated() {
	const apiClient = useApiClient();

	const cb = useCallback(
		(cb: () => void) => {
			apiClient.addEventListener('auth-change', cb);

			return () => {
				apiClient.removeEventListener('auth-change', cb);
			};
		},
		[apiClient],
	);

	const getSnapshot = useCallback(() => {
		return apiClient.isAuthenticated();
	}, [apiClient]);

	return useSyncExternalStore(cb, getSnapshot);
}
