import { Box, Button, Heading, HStack, Text } from '@chakra-ui/react';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, Ban, Clock, DatabaseBackup, Globe } from 'lucide-react';
import { useMemo, useState } from 'react';
import { QueryTimeline } from '../../components/dashboard/QueryTimeline';
import { RecentActivity } from '../../components/dashboard/RecentActivity';
import { StatCard } from '../../components/dashboard/StatCard';
import { TopDonutChart } from '../../components/dashboard/TopDonutChart';
import { useApiClient } from '../../contexts/ApiClientContext';
import { useUptime } from '../../hooks/useUptime';
import type { TopRange } from '../../lib/api/stats';

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

	const { data: liveData } = useQuery({
		queryKey: ['live-stats'],
		queryFn: async () => apiClient.stats.live(),
		// we only fetch this for the uptime, so we can get away with a long stale time.
		staleTime: 1000 * 60 * 60,
	});

	const { data: topData, isPending: topLoading } = useQuery({
		queryKey: ['top-stats', range],
		queryFn: async () => apiClient.stats.top(range),
		refetchInterval: 30 * 1000,
	});

	const { data: timelineData, isPending: timelineLoading } = useQuery({
		queryKey: ['timeline-stats', range],
		queryFn: async () => apiClient.stats.timeline(range),
		refetchInterval: 30 * 1000,
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

	const uptime = useUptime(liveData?.live_since);

	return (
		<Box>
			<HStack justify='space-between' mb='8'>
				<Box>
					<Heading size='lg' mb='2'>
						Dashboard
					</Heading>
					<HStack gap='2'>
						<Box position='relative'>
							<Box w='2.5' h='2.5' borderRadius='full' bg='status.success' />
							<Box
								position='absolute'
								inset='0'
								borderRadius='full'
								bg='status.success'
								className='animate-pulse-glow'
							/>
						</Box>
						<Text color='fg.muted' fontSize='sm'>
							Uptime: {uptime.text}
						</Text>
					</HStack>
				</Box>
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
							onClick={() => setRange(opt.value)}
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
				gap='6'
				mb='8'
			>
				<StatCard
					label='Total Queries'
					value={totals.total}
					icon={Globe}
					accentColor='accent'
				/>
				<StatCard
					label='Queries Blocked'
					value={totals.blocked}
					icon={Ban}
					accentColor='accent'
				/>
				<StatCard
					label='Total Errors'
					value={totals.errors}
					icon={AlertCircle}
					accentColor='accent'
				/>
				<StatCard
					label='Queries Cached'
					value={totals.cached}
					icon={DatabaseBackup}
					accentColor='accent'
				/>
				<StatCard
					label='Avg Response'
					value={`${avgResponse} ms`}
					icon={Clock}
					accentColor='accent'
				/>
			</Box>

			<Box mb='8'>
				<QueryTimeline
					range={range}
					data={timelineData?.buckets ?? []}
					loading={timelineLoading}
				/>
			</Box>

			<Box
				display='grid'
				gridTemplateColumns={{ base: '1fr', md: 'repeat(3, 1fr)' }}
				gap='6'
				mb='8'
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

			<RecentActivity />
		</Box>
	);
}
