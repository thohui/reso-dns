import { Box, Spinner, Text, VStack } from '@chakra-ui/react';
import type React from 'react';
import { useEffect, useState } from 'react';
import { useApiClient } from '../contexts/ApiClientContext';

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
		return (
			<Box
				height="100vh"
				width="100vw"
				display="flex"
				justifyContent="center"
				alignItems="center"
			>
				<VStack colorPalette="green">
					<Spinner color="colorPalette.600" size="xl" />
					<Text color="colorPalette.600">Loading...</Text>
				</VStack>
			</Box>
		);
	}

	return children;
}
