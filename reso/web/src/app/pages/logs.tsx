import { useState } from 'react';
import { LogsGrid } from '../../components/logs/LogsGrid';
import { useActivities } from '../../hooks/useActivities';

const PAGE_SIZE = 100;

export default function LogsPage() {
	const [page, setPage] = useState(0);
	const activities = useActivities(PAGE_SIZE, page * PAGE_SIZE);

	const data = activities?.data;
	const total = data?.total ?? 0;
	const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

	return (
		<LogsGrid
			activities={data?.items ?? []}
			page={page}
			totalPages={totalPages}
			total={total}
			onPageChange={setPage}
		/>
	);
}

