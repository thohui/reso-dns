import {
	Box,
	Button,
	HStack,
	Icon,
	Input,
	Table,
	Text,
} from '@chakra-ui/react';
import {
	createColumnHelper,
	flexRender,
	getCoreRowModel,
	type SortingState,
	type Updater,
	useReactTable,
} from '@tanstack/react-table';
import {
	ChevronDown,
	ChevronsUpDown,
	ChevronUp,
	Clock,
	Search,
} from 'lucide-react';

import { GridPage } from '@/components/GridPage';
import { ProtocolBadge } from '@/components/ProtocolBadge';
import { RecordTypeBadge } from '@/components/RecordTypeBadge';
import { StatusBadge } from '@/components/StatusBadge';
import {
	type Activity,
	type ActivityListFilter,
	type SortColumn,
	type SortDir,
} from '@/lib/api/activity';
import { getStatusInfo } from '@/lib/status-info';
import { formatDuration, formatTimestamp } from '@/lib/time';
import React, { useCallback, useMemo, useState } from 'react';
import { ActivityDetailDrawer } from './ActivityDetailDrawer';

const columnHelper = createColumnHelper<Activity>();

function buildColumns() {
	return [
		columnHelper.accessor('timestamp', {
			header: 'Time',
			enableSorting: true,
			cell: (info) => (
				<Table.Cell
					py='3'
					px='4'
					fontFamily="'Mozilla Text', sans-serif"
					fontSize='sm'
					color='fg.muted'
				>
					{formatTimestamp(info.getValue())}
				</Table.Cell>
			),
		}),
		columnHelper.display({
			id: 'status',
			header: 'Status',
			enableSorting: false,
			cell: ({ row }) => {
				const statusInfo = getStatusInfo(row.original);
				return (
					<Table.Cell py='3' px='4'>
						<StatusBadge statusInfo={statusInfo} size='sm' />
					</Table.Cell>
				);
			},
		}),
		columnHelper.accessor('qname', {
			header: 'Domain',
			enableSorting: true,
			cell: (info) => {
				const activity = info.row.original;
				return (
					<Table.Cell py='3' px='4'>
						<HStack gap='2'>
							<Text
								fontFamily="'Mozilla Text', sans-serif"
								fontSize='sm'
								whiteSpace='nowrap'
								overflow='hidden'
								textOverflow='ellipsis'
							>
								{activity.qname || '-'}
							</Text>
							{activity.qtype !== null && (
								<RecordTypeBadge recordType={activity.qtype} size='md' />
							)}
						</HStack>
					</Table.Cell>
				);
			},
		}),
		columnHelper.accessor('client', {
			header: 'Client',
			enableSorting: true,
			cell: (info) => (
				<Table.Cell
					py='3'
					px='4'
					fontFamily="'Mozilla Text', sans-serif"
					fontSize='sm'
					color='fg.muted'
				>
					{info.getValue() || 'unknown'}
				</Table.Cell>
			),
		}),
		columnHelper.accessor('transport', {
			header: 'Protocol',
			enableSorting: false,
			cell: ({ getValue }) => (
				<Table.Cell py='3' px='4'>
					<ProtocolBadge protocol={getValue()} size='md' />
				</Table.Cell>
			),
		}),
		columnHelper.display({
			id: 'detail',
			header: 'Detail',
			enableSorting: false,
			cell: ({ row }) => {
				const statusInfo = getStatusInfo(row.original);
				return (
					<Table.Cell py='3' px='4'>
						<Text
							fontSize='xs'
							color={statusInfo.text ? statusInfo.color : 'fg.faint'}
							truncate
							maxW='200px'
						>
							{statusInfo.text || '-'}
						</Text>
					</Table.Cell>
				);
			},
		}),
		columnHelper.accessor('duration', {
			header: 'Duration',
			enableSorting: true,
			cell: (info) => {
				const ms = info.getValue() as number;

				let color = 'fg.muted';

				if (ms > 1000) color = 'status.error';
				else if (ms > 100) color = 'status.warn';

				return (
					<Table.Cell py='3' px='4' textAlign='right'>
						<HStack gap='1' justify='flex-end'>
							<Icon as={Clock} boxSize='3' color='fg.subtle' />
							<Text
								fontFamily="'Mozilla Text', sans-serif"
								fontSize='sm'
								color={color}
							>
								{formatDuration(ms)}
							</Text>
						</HStack>
					</Table.Cell>
				);
			},
		}),
	];
}

type FilterPreset = 'all' | 'blocked' | 'cached' | 'rate_limited' | 'errors';

const FILTER_PRESETS: {
	key: FilterPreset;
	label: string;
	color?: string;
	filter: ActivityListFilter;
}[] = [
	{ key: 'all', label: 'All', filter: {} },
	{
		key: 'blocked',
		label: 'Blocked',
		color: 'status.blocked',
		filter: { blocked: true },
	},
	{
		key: 'cached',
		label: 'Cached',
		color: 'status.cached',
		filter: { cache_hit: true },
	},
	{
		key: 'rate_limited',
		label: 'Rate Limited',
		color: 'status.rate_limited',
		filter: { rate_limited: true },
	},
	{
		key: 'errors',
		label: 'Errors',
		color: 'status.error',
		filter: { error_only: true },
	},
];

function getActivePreset(filter: ActivityListFilter): FilterPreset {
	if (filter.blocked) return 'blocked';
	if (filter.cache_hit) return 'cached';
	if (filter.rate_limited) return 'rate_limited';
	if (filter.error_only) return 'errors';
	return 'all';
}

export type SearchField = 'qname' | 'client';

const SEARCH_FIELD_LABELS: Record<SearchField, string> = {
	qname: 'domain',
	client: 'client',
};

interface LogsGridProps {
	activities: Activity[];
	page: number;
	totalPages: number | null;
	total: number | null;
	hasMore?: boolean;
	onPageChange: (page: number) => void;
	presetFilter: ActivityListFilter;
	onPresetChange: (filter: ActivityListFilter) => void;
	sort: SortColumn;
	dir: SortDir;
	onSortChange: (col: SortColumn, dir: SortDir) => void;
	isLoading: boolean;
	searchField: SearchField;
	searchValue: string;
	onSearchFieldChange: (field: SearchField) => void;
	onSearchChange: (value: string) => void;
}

export function LogsGrid({
	activities,
	page,
	totalPages,
	total,
	hasMore,
	onPageChange,
	presetFilter,
	onPresetChange,
	sort,
	dir,
	onSortChange,
	isLoading,
	searchField,
	searchValue,
	onSearchFieldChange,
	onSearchChange,
}: LogsGridProps) {
	const [selectedActivity, setSelectedActivity] = useState<Activity | null>(
		null,
	);

	const activePreset = getActivePreset(presetFilter);
	const columns = useMemo(() => buildColumns(), []);

	const sorting: SortingState = useMemo(
		() => [{ id: sort, desc: dir === 'desc' }],
		[sort, dir],
	);

	function handleSortingChange(updater: Updater<SortingState>) {
		const next = typeof updater === 'function' ? updater(sorting) : updater;
		if (next.length > 0) {
			onSortChange(next[0].id as SortColumn, next[0].desc ? 'desc' : 'asc');
		}
	}

	const table = useReactTable({
		data: activities,
		columns,
		state: { sorting },
		manualSorting: true,
		manualPagination: true,
		pageCount: totalPages ?? undefined,
		enableSortingRemoval: false,
		sortDescFirst: true,
		onSortingChange: handleSortingChange,
		getCoreRowModel: getCoreRowModel(),
	});

	const handleRowKeyDown = useCallback(
		(e: React.KeyboardEvent<HTMLTableRowElement>, activity: Activity) => {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				setSelectedActivity(activity);
			} else if (e.key === 'ArrowDown') {
				e.preventDefault();
				(e.currentTarget.nextElementSibling as HTMLElement | null)?.focus();
			} else if (e.key === 'ArrowUp') {
				e.preventDefault();
				(e.currentTarget.previousElementSibling as HTMLElement | null)?.focus();
			}
		},
		[],
	);

	const toolbar = (
		<HStack gap='2' justify='space-between' flexWrap='wrap'>
			<HStack gap='2' flexWrap='wrap'>
				{FILTER_PRESETS.map(({ key, label, color, filter }) => {
					const active = activePreset === key;
					return (
						<Button
							key={key}
							variant='ghost'
							onClick={() => onPresetChange(filter)}
							px='3'
							py='1.5'
							minH='auto'
							h='auto'
							borderRadius='full'
							fontSize='xs'
							fontWeight='500'
							cursor='pointer'
							transition='all 0.15s ease'
							borderWidth='1px'
							borderColor={active ? (color ?? 'fg.subtle') : 'border'}
							bg={active ? 'bg.subtle' : 'transparent'}
							color={active ? (color ?? 'fg') : 'fg.muted'}
							_hover={{ bg: 'bg.subtle', borderColor: color ?? 'fg.subtle' }}
							aria-pressed={active}
							aria-label={`${label} filter`}
						>
							<HStack gap='1.5'>
								{color && (
									<Box
										w='1.5'
										h='1.5'
										borderRadius='full'
										bg={color}
										opacity={active ? 1 : 0.5}
									/>
								)}
								<Text fontSize='xs' lineHeight='1'>
									{label}
								</Text>
							</HStack>
						</Button>
					);
				})}
			</HStack>

			<HStack
				borderWidth='1px'
				borderColor={searchValue ? 'fg.subtle' : 'border'}
				borderRadius='full'
				px='3'
				py='1.5'
				gap='1.5'
				transition='border-color 0.15s'
			>
				<Icon as={Search} boxSize='3' color='fg.faint' flexShrink={0} />
				<Input
					variant='subtle'
					value={searchValue}
					onChange={(e) => onSearchChange(e.target.value)}
					placeholder={`search ${SEARCH_FIELD_LABELS[searchField]}...`}
					fontSize='xs'
					lineHeight='1'
					border='hidden'
					p='0'
					h='auto'
					minW='32'
					bg='transparent'
					fontFamily="'Mozilla Text', sans-serif"
				/>
				<Box w='1px' h='3' bg='border' flexShrink={0} />
				<Text
					as='button'
					fontSize='xs'
					color='fg.muted'
					cursor='pointer'
					onClick={() =>
						onSearchFieldChange(searchField === 'qname' ? 'client' : 'qname')
					}
					_hover={{ color: 'fg' }}
					transition='color 0.15s'
					whiteSpace='nowrap'
					flexShrink={0}
				>
					{SEARCH_FIELD_LABELS[searchField]}
				</Text>
			</HStack>
		</HStack>
	);

	return (
		<Box>
			<GridPage
				toolbar={toolbar}
				isLoading={isLoading}
				page={page}
				totalPages={totalPages}
				total={total}
				totalLabel='total entries'
				hasMore={hasMore}
				onPageChange={onPageChange}
			>
				<Table.Root size='sm'>
					<Table.Header>
						{table.getHeaderGroups().map((headerGroup) => (
							<Table.Row key={headerGroup.id} bg='bg.subtle'>
								{headerGroup.headers.map((header) => {
									const canSort = header.column.getCanSort();
									const sorted = header.column.getIsSorted();
									const SortIcon =
										sorted === 'desc'
											? ChevronDown
											: sorted === 'asc'
												? ChevronUp
												: ChevronsUpDown;

									return (
										<Table.ColumnHeader
											key={header.id}
											py='3'
											px='4'
											fontSize='xs'
											textTransform='uppercase'
											letterSpacing='wider'
											color={sorted ? 'fg' : 'fg.muted'}
											cursor={canSort ? 'pointer' : 'default'}
											userSelect={canSort ? 'none' : undefined}
											_hover={canSort ? { color: 'fg' } : undefined}
											onClick={
												canSort
													? header.column.getToggleSortingHandler()
													: undefined
											}
											textAlign={
												header.column.id === 'duration' ? 'right' : undefined
											}
										>
											<HStack
												gap='1'
												justify={
													header.column.id === 'duration'
														? 'flex-end'
														: 'flex-start'
												}
											>
												<Text>
													{flexRender(
														header.column.columnDef.header,
														header.getContext(),
													)}
												</Text>
												{canSort && (
													<Icon
														as={SortIcon}
														boxSize='3'
														opacity={sorted ? 1 : 0.4}
													/>
												)}
											</HStack>
										</Table.ColumnHeader>
									);
								})}
							</Table.Row>
						))}
					</Table.Header>
					<Table.Body>
						{table.getRowModel().rows.map((row) => (
							<Table.Row
								key={row.id}
								bg='bg.panel'
								borderColor='border'
								_hover={{ bg: 'bg.subtle' }}
								_focus={{ bg: 'bg.subtle', outline: 'none' }}
								transition='background 0.15s'
								cursor='pointer'
								tabIndex={0}
								onClick={() => setSelectedActivity(row.original)}
								onKeyDown={(e) => handleRowKeyDown(e, row.original)}
							>
								{row.getVisibleCells().map((cell) => (
									<React.Fragment key={cell.id}>
										{flexRender(cell.column.columnDef.cell, cell.getContext())}
									</React.Fragment>
								))}
							</Table.Row>
						))}
						{table.getRowModel().rows.length === 0 && !isLoading && (
							<Table.Row bg='bg.panel'>
								<Table.Cell
									colSpan={columns.length}
									py='8'
									textAlign='center'
									color='fg.muted'
								>
									No entries match this filter
								</Table.Cell>
							</Table.Row>
						)}
					</Table.Body>
				</Table.Root>
			</GridPage>

			<ActivityDetailDrawer
				activity={selectedActivity}
				open={selectedActivity !== null}
				onClose={() => setSelectedActivity(null)}
			/>
		</Box>
	);
}
