import {
	Badge,
	Box,
	Heading,
	HStack,
	Icon,
	Table,
	Tabs,
	Text,
	VStack,
} from '@chakra-ui/react';
import {
	AlertTriangle,
	CheckCircle,
	Clock,
	ShieldOff,
	XCircle,
	Zap,
} from 'lucide-react';
import { useState } from 'react';
import {
	type Activity,
	type QueryActivity,
	RCODE_LABELS,
	RECORD_TYPES,
	TRANSPORT_LABELS,
} from '../../lib/api/activity';

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

function getStatusInfo(activity: Activity) {
	if (activity.kind === 'error') {
		return {
			label: 'ERROR',
			color: 'red' as const,
			icon: XCircle,
			tokenColor: 'status.error',
		};
	}
	const q = activity as QueryActivity;
	if (q.d.blocked) {
		return {
			label: 'BLOCKED',
			color: 'orange' as const,
			icon: ShieldOff,
			tokenColor: 'status.blocked',
		};
	}
	if (q.d.cache_hit) {
		return {
			label: 'CACHED',
			color: 'blue' as const,
			icon: Zap,
			tokenColor: 'status.cached',
		};
	}
	if (q.d.rcode !== 0) {
		return {
			label: RCODE_LABELS[q.d.rcode] || `RCODE:${q.d.rcode}`,
			color: 'yellow' as const,
			icon: AlertTriangle,
			tokenColor: 'status.warn',
		};
	}
	return {
		label: 'OK',
		color: 'green' as const,
		icon: CheckCircle,
		tokenColor: 'status.success',
	};
}

function LogDetailRow({ activity }: { activity: Activity }) {
	const status = getStatusInfo(activity);

	return (
		<Table.Row
			bg="bg.panel"
			borderColor="border"
			_hover={{ bg: 'bg.subtle' }}
			transition="background 0.15s"
		>
			<Table.Cell
				py="3"
				px="4"
				fontFamily="mono"
				fontSize="sm"
				color="fg.muted"
			>
				{formatTimestamp(activity.timestamp)}
			</Table.Cell>

			<Table.Cell py="3" px="4">
				<HStack gap="2">
					<Icon as={status.icon} boxSize="3.5" color={status.tokenColor} />
					<Badge colorPalette={status.color} size="sm" variant="subtle">
						{status.label}
					</Badge>
				</HStack>
			</Table.Cell>

			<Table.Cell py="3" px="4">
				<HStack gap="2">
					<Text fontFamily="mono" fontSize="sm">
						{activity.qname || '-'}
					</Text>
					{activity.qtype && (
						<Badge
							colorPalette="gray"
							size="sm"
							variant="outline"
							color="white"
							fontFamily="mono"
						>
							{RECORD_TYPES[activity.qtype] || 'Unknown'}
						</Badge>
					)}
				</HStack>
			</Table.Cell>

			<Table.Cell
				py="3"
				px="4"
				fontFamily="mono"
				fontSize="sm"
				color="fg.muted"
			>
				{activity.client || 'unknown'}
			</Table.Cell>

			<Table.Cell py="3" px="4">
				<Badge colorPalette="gray" size="sm" variant="subtle">
					{TRANSPORT_LABELS[activity.transport] || `T:${activity.transport}`}
				</Badge>
			</Table.Cell>

			<Table.Cell py="3" px="4" textAlign="right">
				<HStack gap="1" justify="flex-end">
					<Icon as={Clock} boxSize="3" color="fg.subtle" />
					<Text
						fontFamily="mono"
						fontSize="sm"
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
			<HStack justify="space-between" mb="6" align="flex-end">
				<VStack align="flex-start" gap="1">
					<Heading size="lg">DNS Query Logs</Heading>
					<Text color="fg.muted" fontSize="sm">
						{activities.length} total entries
					</Text>
				</VStack>
			</HStack>

			<Tabs.Root
				defaultValue="all"
				mb="6"
				onValueChange={(e) => setFilter(e.value)}
			>
				<Tabs.List bg="bg.panel" borderRadius="lg" p="1" gap="1">
					<Tabs.Trigger
						value="all"
						px="4"
						py="2"
						color="fg.muted"
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						All ({counts.all})
					</Tabs.Trigger>
					<Tabs.Trigger
						value="queries"
						px="4"
						py="2"
						color="fg.muted"
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Queries ({counts.queries})
					</Tabs.Trigger>
					<Tabs.Trigger
						value="blocked"
						px="4"
						py="2"
						color="fg.muted"
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Blocked ({counts.blocked})
					</Tabs.Trigger>
					<Tabs.Trigger
						value="cached"
						px="4"
						py="2"
						color="fg.muted"
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Cached ({counts.cached})
					</Tabs.Trigger>
					<Tabs.Trigger
						value="errors"
						px="4"
						py="2"
						color="fg.muted"
						_selected={{ bg: 'bg.subtle', color: 'fg' }}
					>
						Errors ({counts.errors})
					</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>

			<Box
				bg="bg.panel"
				borderRadius="lg"
				borderWidth="1px"
				borderColor="border"
				overflow="hidden"
			>
				<Box overflowX="auto">
					<Table.Root size="sm">
						<Table.Header>
							<Table.Row bg="bg.subtle">
								<Table.ColumnHeader
									color="fg.muted"
									py="3"
									px="4"
									fontSize="xs"
									textTransform="uppercase"
									letterSpacing="wider"
								>
									Time
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color="fg.muted"
									py="3"
									px="4"
									fontSize="xs"
									textTransform="uppercase"
									letterSpacing="wider"
								>
									Status
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color="fg.muted"
									py="3"
									px="4"
									fontSize="xs"
									textTransform="uppercase"
									letterSpacing="wider"
								>
									Domain
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color="fg.muted"
									py="3"
									px="4"
									fontSize="xs"
									textTransform="uppercase"
									letterSpacing="wider"
								>
									Client
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color="fg.muted"
									py="3"
									px="4"
									fontSize="xs"
									textTransform="uppercase"
									letterSpacing="wider"
								>
									Protocol
								</Table.ColumnHeader>
								<Table.ColumnHeader
									color="fg.muted"
									py="3"
									px="4"
									fontSize="xs"
									textTransform="uppercase"
									letterSpacing="wider"
									textAlign="right"
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
								/>
							))}
							{filteredActivities.length === 0 && (
								<Table.Row bg="bg.panel">
									<Table.Cell
										colSpan={7}
										py="8"
										textAlign="center"
										color="fg.muted"
									>
										No entries match this filter
									</Table.Cell>
								</Table.Row>
							)}
						</Table.Body>
					</Table.Root>
				</Box>
			</Box>
		</Box>
	);
}
