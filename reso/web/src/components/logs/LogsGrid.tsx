import { getStatusInfo } from '../../lib/status-info';
import {
	Box,
	Heading,
	HStack,
	Icon,
	Table,
	Tabs,
	Text,
	VStack,
} from '@chakra-ui/react';
import { Clock } from 'lucide-react';
import { useCallback, useState } from 'react';
import {
	type Activity,
	getRecordType,
	getTransportLabel,
	type QueryActivity,
} from '../../lib/api/activity';
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
				fontFamily="'JetBrains Mono', monospace"
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
					<Text fontFamily="'JetBrains Mono', monospace" fontSize='sm'>
						{activity.qname || '-'}
					</Text>
					{activity.qtype !== null && (
						<Box
							px='2'
							py='0.5'
							borderRadius='md'
							fontSize='xs'
							fontWeight='500'
							fontFamily="'JetBrains Mono', monospace"
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
				fontFamily="'JetBrains Mono', monospace"
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
					fontFamily="'JetBrains Mono', monospace"
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
						fontFamily="'JetBrains Mono', monospace"
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

export function LogsGrid({ activities }: { activities: Activity[] }) {
	const [filter, setFilter] = useState('all');
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
		return true;
	});

	const counts = {
		all: activities.length,
		queries: activities.filter(
			(a) => a.kind === 'query' && !(a as QueryActivity).d.blocked,
		).length,
		blocked: activities.filter(
			(a) => a.kind === 'query' && (a as QueryActivity).d.blocked,
		).length,
		errors: activities.filter((a) => a.kind === 'error').length,
		cached: activities.filter(
			(a) => a.kind === 'query' && (a as QueryActivity).d.cache_hit,
		).length,
	};

	return (
		<Box>
			<HStack justify='space-between' mb='6' align='flex-end'>
				<VStack align='flex-start' gap='1'>
					<Heading size='lg'>DNS Query Logs</Heading>
					<Text color='fg.muted' fontSize='sm'>
						{activities.length} total entries
					</Text>
				</VStack>
			</HStack>

			<Tabs.Root
				defaultValue='all'
				mb='6'
				onValueChange={(e) => setFilter(e.value)}
			>
				<Tabs.List bg='bg.panel' borderRadius='lg' p='1' gap='1'>
					<Tabs.Trigger
						value='all'
						px='4'
						py='2'
						color='fg.muted'
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						All ({counts.all})
					</Tabs.Trigger>
					<Tabs.Trigger
						value='queries'
						px='4'
						py='2'
						color='fg.muted'
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Queries ({counts.queries})
					</Tabs.Trigger>
					<Tabs.Trigger
						value='blocked'
						px='4'
						py='2'
						color='fg.muted'
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Blocked ({counts.blocked})
					</Tabs.Trigger>
					<Tabs.Trigger
						value='cached'
						px='4'
						py='2'
						color='fg.muted'
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Cached ({counts.cached})
					</Tabs.Trigger>
					<Tabs.Trigger
						value='errors'
						px='4'
						py='2'
						color='fg.muted'
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Errors ({counts.errors})
					</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>

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

			<ActivityDetailDrawer
				activity={selectedActivity}
				open={selectedActivity !== null}
				onClose={() => setSelectedActivity(null)}
			/>
		</Box>
	);
}
