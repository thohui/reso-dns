import { LogsGrid } from '../../components/logs/LogsGrid';
import { useActivities } from '../../hooks/useActivities';

export default function LogsPage() {
	const activities = useActivities(100, 0);

	return <LogsGrid activities={activities?.data?.items ?? []} />;
}
