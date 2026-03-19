import { Heading } from '@chakra-ui/react';
import { useRef, useState } from 'react';
import { LogsGrid, type SearchField } from '../../components/logs/LogsGrid';
import { useActivities } from '../../hooks/useActivities';
import { useDebounce } from '../../hooks/useDebounce';
import type {
	ActivityListFilter,
	SortColumn,
	SortDir,
} from '../../lib/api/activity';

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
			<Heading size='lg' mb={8}>
				Logs
			</Heading>
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
