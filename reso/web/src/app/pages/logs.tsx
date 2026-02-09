import { useQuery } from '@tanstack/react-query';
import { LogsGrid } from '../../components/logs/LogsGrid';
import { useApiClient } from '../../contexts/ApiClientContext';

export default function LogsPage() {
	const apiClient = useApiClient();

	const { data } = useQuery({
		queryKey: ['actitivies'],
		queryFn: async () => {
			return apiClient.activities.list({ top: 100, skip: 0 });
		},
	});
	return <LogsGrid activities={data?.items ?? []} />;
}
