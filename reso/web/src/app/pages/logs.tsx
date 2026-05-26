import { StatCard } from '@/components/dashboard/StatCard';
import { LogsGrid, type SearchField } from '@/components/logs/LogsGrid';
import { useActivities } from '@/hooks/logs/useActivities';
import { useLiveStats } from '@/hooks/dashboard/useLiveStats';
import { useDebounce } from '@/hooks/useDebounce';
import type {
	ActivityListFilter,
	SortColumn,
	SortDir,
} from '@/lib/api/activity';
import { Grid } from '@chakra-ui/react';
import { AlertCircle, Ban, DatabaseBackup, Globe } from 'lucide-react';
import { useRef, useState } from 'react';

const PAGE_SIZE = 50;

export default function LogsPage() {
	const [page, setPage] = useState(0);
	const [presetFilter, setPresetFilter] = useState<ActivityListFilter>({});
	const [sort, setSort] = useState<SortColumn>('timestamp');
	const [dir, setDir] = useState<SortDir>('desc');
	const [searchField, setSearchField] = useState<SearchField>('qname');
	const [searchValue, setSearchValue] = useState('');
	const cachedTotal = useRef<number | null>(null);

	const debouncedSearch = useDebounce(searchValue, 300);

	const filter: ActivityListFilter = {
		...presetFilter,
		...(debouncedSearch !== '' ? { [searchField]: debouncedSearch } : {}),
	};

	const needsCount = page === 0;

	const { data, isFetching: isLoading } = useActivities({
		top: PAGE_SIZE,
		skip: page * PAGE_SIZE,
		filter,
		sort,
		dir,
		count: needsCount,
	});

	if (data?.total != null) {
		cachedTotal.current = data.total;
	}

	const { data: liveData } = useLiveStats();

	const total = data?.total ?? cachedTotal.current;
	const totalPages =
		total != null ? Math.max(1, Math.ceil(total / PAGE_SIZE)) : null;

	function handlePresetChange(next: ActivityListFilter) {
		setPresetFilter(next);
		setPage(0);
	}

	function handleSortChange(col: SortColumn, nextDir: SortDir) {
		setSort(col);
		setDir(nextDir);
		setPage(0);
	}

	function handleSearchChange(value: string) {
		setSearchValue(value);
		setPage(0);
	}

	function handleSearchFieldChange(field: SearchField) {
		setSearchField(field);
		if (searchValue) setPage(0);
	}

	return (
		<>
			<Grid templateColumns='repeat(4, 1fr)' gap='4' mb='6'>
				<StatCard
					label='Total Queries'
					value={liveData?.total ?? '—'}
					icon={Globe}
					accentColor='status.info'
				/>
				<StatCard
					label='Blocked'
					value={liveData?.blocked ?? '—'}
					icon={Ban}
					accentColor='status.error'
				/>
				<StatCard
					label='Cached'
					value={liveData?.cached ?? '—'}
					icon={DatabaseBackup}
					accentColor='status.success'
				/>
				<StatCard
					label='Errors'
					value={liveData?.errors ?? '—'}
					icon={AlertCircle}
					accentColor='status.warn'
				/>
			</Grid>
			<LogsGrid
				isLoading={isLoading}
				activities={data?.items ?? []}
				page={page}
				totalPages={totalPages}
				total={total}
				onPageChange={setPage}
				presetFilter={presetFilter}
				onPresetChange={handlePresetChange}
				sort={sort}
				dir={dir}
				onSortChange={handleSortChange}
				searchField={searchField}
				searchValue={searchValue}
				onSearchFieldChange={handleSearchFieldChange}
				onSearchChange={handleSearchChange}
			/>
		</>
	);
}
