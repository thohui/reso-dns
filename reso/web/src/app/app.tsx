import { ChakraProvider } from '@chakra-ui/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Initializer } from '../components/Initializer';
import { ApiClientContext } from '../contexts/ApiClientContext';
import { ApiClient } from '../lib/api/client';
import { system } from '../lib/theme';
import { AppRouter } from './router';

const apiClient = new ApiClient();
const queryClient = new QueryClient();

function App() {
	return (
		<ChakraProvider value={system}>
			<QueryClientProvider client={queryClient}>
				<ApiClientContext.Provider value={apiClient}>
					<Initializer>
						<AppRouter />
					</Initializer>
				</ApiClientContext.Provider>
			</QueryClientProvider>
		</ChakraProvider>
	);
}

export default App;
