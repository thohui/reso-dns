import type React from 'react';
import { useEffect, useState } from 'react';
import { useApiClient } from '@/contexts/ApiClientContext';
import { PageLoader } from './PageLoader';

export function Initializer({ children }: React.PropsWithChildren) {
	const [loading, setLoading] = useState(true);

	const apiClient = useApiClient();
	useEffect(() => {
		apiClient
			.initialize()
			.catch((e) => console.log('initializing failed:', e))
			.finally(() => setLoading(false));
	}, [apiClient]);

	if (loading) {
		return <PageLoader />;
	}

	return children;
}
