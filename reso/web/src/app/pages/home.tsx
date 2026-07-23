import { Box, Button, HStack } from '@chakra-ui/react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { AlertCircle, Ban, Clock, DatabaseBackup, Globe } from 'lucide-react';
import { useMemo, useState, useTransition } from 'react';
import { QueryTimeline } from '@/components/dashboard/QueryTimeline';
import { RecentActivity } from '@/components/dashboard/RecentActivity';
import { StatCard } from '@/components/dashboard/StatCard';
import { TopDonutChart } from '@/components/dashboard/TopDonutChart';
import { useApiClient } from '@/contexts/ApiClientContext';
import type { TopRange } from '@/lib/api/stats';

const RANGE_OPTIONS: { value: TopRange; label: string }[] = [
	{ value: '5min', label: '5M' },
	{ value: 'hour', label: '1H' },
	{ value: 'day', label: '24H' },
	{ value: 'week', label: '7D' },
	{ value: 'month', label: '30D' },
	{ value: 'year', label: '365D' },
	{ value: 'all', label: 'All' },
];

export default function HomePage() {
	const apiClient = useApiClient();

	const [range, setRange] = useState<TopRange>('day');
	const [, startTransition] = useTransition();

	const { data: topData, isPending: topLoading } = useQuery({
		queryKey: ['top-stats', range],
		queryFn: async () => apiClient.stats.top(range),
		refetchInterval: 30 * 1000,
		placeholderData: keepPreviousData,
	});

	const { data: timelineData, isPending: timelineLoading } = useQuery({
		queryKey: ['timeline-stats', range],
		queryFn: async () => apiClient.stats.timeline(range),
		refetchInterval: 30 * 1000,
		placeholderData: keepPreviousData,
	});

	const totals = useMemo(() => {
		const buckets = timelineData?.buckets ?? [];
		return buckets.reduce(
			(acc, b) => ({
				total: acc.total + b.total,
				blocked: acc.blocked + b.blocked,
				cached: acc.cached + b.cached,
				errors: acc.errors + b.errors,
				sum_duration: acc.sum_duration + b.sum_duration,
			}),
			{ total: 0, blocked: 0, cached: 0, errors: 0, sum_duration: 0 },
		);
	}, [timelineData]);

	const avgResponse =
		totals.total > 0 ? (totals.sum_duration / totals.total).toFixed() : '0';

	return (
		<Box h='full' display='flex' flexDir='column' gap='4'>
			<HStack justify='flex-end' flexShrink={0}>
				<HStack gap='1'>
					{RANGE_OPTIONS.map((opt) => (
						<Button
							key={opt.value}
							size='xs'
							variant={range === opt.value ? 'solid' : 'ghost'}
							bg={range === opt.value ? 'accent' : undefined}
							color={range === opt.value ? 'white' : 'fg.muted'}
							_hover={{
								bg: range === opt.value ? 'accent.hover' : 'bg.subtle',
							}}
							onClick={() => startTransition(() => setRange(opt.value))}
						>
							{opt.label}
						</Button>
					))}
				</HStack>
			</HStack>

			<Box
				display='grid'
				gridTemplateColumns={{
					base: '1fr',
					md: 'repeat(2, 1fr)',
					lg: 'repeat(5, 1fr)',
				}}
				gap='4'
				flexShrink={0}
			>
				<StatCard
					label='Total Queries'
					value={totals.total}
					icon={Globe}
					accentColor='accent'
					isLoading={timelineLoading}
				/>
				<StatCard
					label='Queries Blocked'
					value={totals.blocked}
					icon={Ban}
					accentColor='accent'
					isLoading={timelineLoading}
				/>
				<StatCard
					label='Total Errors'
					value={totals.errors}
					icon={AlertCircle}
					accentColor='accent'
					isLoading={timelineLoading}
				/>
				<StatCard
					label='Queries Cached'
					value={totals.cached}
					icon={DatabaseBackup}
					accentColor='accent'
					isLoading={timelineLoading}
				/>
				<StatCard
					label='Average Response time'
					value={`${avgResponse} ms`}
					icon={Clock}
					accentColor='accent'
					isLoading={timelineLoading}
				/>
			</Box>

			<Box flex='1' minH='0'>
				<QueryTimeline
					data={timelineData?.buckets ?? []}
					loading={timelineLoading}
				/>
			</Box>

			<Box
				display='grid'
				gridTemplateColumns={{ base: '1fr', md: 'repeat(3, 1fr)' }}
				gap='4'
				flexShrink={0}
			>
				<TopDonutChart
					title='Top Clients'
					data={topData?.clients ?? []}
					loading={topLoading}
				/>
				<TopDonutChart
					title='Top Domains'
					data={topData?.domains ?? []}
					loading={topLoading}
				/>
				<TopDonutChart
					title='Top Blocked'
					data={topData?.blocked_domains ?? []}
					loading={topLoading}
				/>
			</Box>
			<Box flexShrink={0}>
				<RecentActivity />
			</Box>
		</Box>
	);
}
