import {
	Box,
	Button,
	HStack,
	Icon,
	Table,
	Text,
} from '@chakra-ui/react';
import { ChevronLeft, ChevronRight, Clock } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import {
	type Activity,
	getRecordType,
	getTransportLabel,
	type QueryActivity,
} from '../../lib/api/activity';
import { getStatusInfo } from '../../lib/status-info';
import { ActivityDetailDrawer } from './ActivityDetailDrawer';

function formatTimestamp(ts: number): string {
	const d = new Date(ts);
	return d.toLocaleTimeString('en-US', {
		hour12: false,
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
	});
}

function formatDuration(ms: number): string {
	if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
	return `${ms}ms`;
}

function LogDetailRow({
	activity,
	onClick,
	onKeyDown,
}: {
	activity: Activity;
	onClick: () => void;
	onKeyDown: (e: React.KeyboardEvent<HTMLTableRowElement>) => void;
}) {
	const statusInfo = getStatusInfo(activity);

	return (
		<Table.Row
			bg='bg.panel'
			borderColor='border'
			_hover={{ bg: 'bg.subtle' }}
			_focus={{ bg: 'bg.subtle', outline: 'none' }}
			transition='background 0.15s'
			cursor='pointer'
			tabIndex={0}
			onClick={onClick}
			onKeyDown={onKeyDown}
		>
			<Table.Cell
				py='3'
				px='4'
				fontFamily="'Mozilla Text', sans-serif"
				fontSize='sm'
				color='fg.muted'
			>
				{formatTimestamp(activity.timestamp)}
			</Table.Cell>

			<Table.Cell py='3' px='4'>
				<HStack gap='2'>
					<Icon as={statusInfo.icon} boxSize='3.5' color={statusInfo.color} />
					<Box
						px='2.5'
						py='0.5'
						borderRadius='md'
						fontSize='xs'
						fontWeight='600'
						textTransform='uppercase'
						letterSpacing='0.03em'
						bg={statusInfo.bg}
						color={statusInfo.color}
					>
						{statusInfo.label}
					</Box>
				</HStack>
			</Table.Cell>

			<Table.Cell py='3' px='4'>
				<HStack gap='2'>
					<Text fontFamily="'Mozilla Text', sans-serif" fontSize='sm'>
						{activity.qname || '-'}
					</Text>
					{activity.qtype !== null && (
						<Box
							px='2'
							py='0.5'
							borderRadius='md'
							fontSize='xs'
							fontWeight='500'
							fontFamily="'Mozilla Text', sans-serif"
							bg='accent.muted'
							color='accent.fg'
						>
							{getRecordType(activity.qtype)}
						</Box>
					)}
				</HStack>
			</Table.Cell>

			<Table.Cell
				py='3'
				px='4'
				fontFamily="'Mozilla Text', sans-serif"
				fontSize='sm'
				color='fg.muted'
			>
				{activity.client || 'unknown'}
			</Table.Cell>

			<Table.Cell py='3' px='4'>
				<Box
					display='inline-block'
					px='2'
					py='0.5'
					borderRadius='md'
					fontSize='xs'
					fontWeight='500'
					fontFamily="'Mozilla Text', sans-serif"
					bg='accent.muted'
					color='accent.fg'
				>
					{getTransportLabel(activity.transport)}
				</Box>
			</Table.Cell>

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

			<Table.Cell py='3' px='4' textAlign='right'>
				<HStack gap='1' justify='flex-end'>
					<Icon as={Clock} boxSize='3' color='fg.subtle' />
					<Text
						fontFamily="'Mozilla Text', sans-serif"
						fontSize='sm'
						color={
							activity.duration > 1000
								? 'status.error'
								: activity.duration > 100
									? 'status.warn'
									: 'fg.muted'
						}
					>
						{formatDuration(activity.duration)}
					</Text>
				</HStack>
			</Table.Cell>
		</Table.Row>
	);
}


type ActivityFilter = 'all' | 'queries' | 'blocked' | 'errors' | 'cached' | 'rate_limited';

const FILTERS: { key: ActivityFilter; label: string; color?: string; }[] = [
	{ key: 'all', label: 'All' },
	{ key: 'queries', label: 'Queries', color: 'status.success' },
	{ key: 'blocked', label: 'Blocked', color: 'status.blocked' },
	{ key: 'cached', label: 'Cached', color: 'status.cached' },
	{ key: 'rate_limited', label: 'Rate Limited', color: 'status.rate_limited' },
	{ key: 'errors', label: 'Errors', color: 'status.error' },
];

interface LogsGridProps {
	activities: Activity[];
	page: number;
	totalPages: number;
	total: number;
	onPageChange: (page: number) => void;
}

export function LogsGrid({ activities, page, totalPages, total, onPageChange }: LogsGridProps) {
	const [filter, setFilter] = useState<ActivityFilter>('all');
	const [selectedActivity, setSelectedActivity] = useState<Activity | null>(
		null,
	);
	const handleRowKeyDown = useCallback(
		(e: React.KeyboardEvent<HTMLTableRowElement>, activity: Activity) => {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				setSelectedActivity(activity);
			} else if (e.key === 'ArrowDown') {
				e.preventDefault();
				const next = e.currentTarget.nextElementSibling as HTMLElement | null;
				next?.focus();
			} else if (e.key === 'ArrowUp') {
				e.preventDefault();
				const prev = e.currentTarget
					.previousElementSibling as HTMLElement | null;
				prev?.focus();
			}
		},
		[],
	);

	const filteredActivities = activities.filter((a) => {
		if (filter === 'all') return true;
		if (filter === 'queries')
			return a.kind === 'query' && !(a as QueryActivity).d.blocked;
		if (filter === 'blocked')
			return a.kind === 'query' && (a as QueryActivity).d.blocked;
		if (filter === 'errors') return a.kind === 'error';
		if (filter === 'cached')
			return a.kind === 'query' && (a as QueryActivity).d.cache_hit;
		if (filter === 'rate_limited')
			return a.kind === 'query' && (a as QueryActivity).d.rate_limited;
		return true;
	});

	const counts = useMemo(() => {
		let queries = 0, blocked = 0, errors = 0, cached = 0, rate_limited = 0;
		for (const a of activities) {
			if (a.kind === 'error') { errors++; continue; }
			if (a.kind === 'query') {
				const d = (a as QueryActivity).d;
				if (d.blocked) blocked++; else queries++;
				if (d.cache_hit) cached++;
				if (d.rate_limited) rate_limited++;
			}
		}
		return { all: activities.length, queries, blocked, errors, cached, rate_limited };
	}, [activities]);

	return (
		<Box>
			<HStack gap='2' mb='4' flexWrap='wrap'>
				{FILTERS.map(({ key, label, color }) => {
					const active = filter === key;
					return (
						<Box
							key={key}
							as='button'
							onClick={() => setFilter(key)}
							px='3'
							py='1.5'
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
								<Text fontSize='xs' color={active ? 'fg.subtle' : 'fg.faint'} lineHeight='1'>
									{counts[key]}
								</Text>
							</HStack>
						</Box>
					);
				})}
			</HStack>

			<Box
				bg='bg.panel'
				borderRadius='lg'
				borderWidth='1px'
				borderColor='border'
				overflow='hidden'
			>
				<Box overflowX='auto'>
					<Table.Root size='sm'>
						<Table.Header>
							<Table.Row bg='bg.subtle'>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
								>
									Time
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
								>
									Status
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
								>
									Domain
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
								>
									Client
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
								>
									Protocol
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
								>
									Detail
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color='fg.muted'
									py='3'
									px='4'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='wider'
									textAlign='right'
								>
									Duration
								</Table.ColumnHeader>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{filteredActivities.map((activity, i) => (
								<LogDetailRow
									key={`${activity.timestamp}-${i}`}
									activity={activity}
									onClick={() => setSelectedActivity(activity)}
									onKeyDown={(e) => handleRowKeyDown(e, activity)}
								/>
							))}
							{filteredActivities.length === 0 && (
								<Table.Row bg='bg.panel'>
									<Table.Cell
										colSpan={7}
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
				</Box>
			</Box>

			{totalPages > 1 && (
				<HStack justify='space-between' mt='4' px='1'>
					<Text fontSize='xs' color='fg.muted'>
						{total.toLocaleString()} total entries
					</Text>
					<HStack gap='2'>
						<Button
							size='xs'
							variant='ghost'
							color='fg.muted'
							_hover={{ bg: 'bg.subtle' }}
							disabled={page === 0}
							onClick={() => onPageChange(page - 1)}
						>
							<Icon as={ChevronLeft} boxSize='3.5' />
							Prev
						</Button>
						<Text fontSize='xs' color='fg.muted'>
							{page + 1} / {totalPages}
						</Text>
						<Button
							size='xs'
							variant='ghost'
							color='fg.muted'
							_hover={{ bg: 'bg.subtle' }}
							disabled={page >= totalPages - 1}
							onClick={() => onPageChange(page + 1)}
						>
							Next
							<Icon as={ChevronRight} boxSize='3.5' />
						</Button>
					</HStack>
				</HStack>
			)}

			<ActivityDetailDrawer
				activity={selectedActivity}
				open={selectedActivity !== null}
				onClose={() => setSelectedActivity(null)}
			/>
		</Box>
	);
}
