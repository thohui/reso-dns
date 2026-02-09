import { Box, Heading, HStack, Text } from '@chakra-ui/react';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, Ban, Clock, DatabaseBackup, Globe } from 'lucide-react';
import { RecentActivity } from '../../components/dashboard/RecentActivity';
import { StatCard } from '../../components/dashboard/StatCard';
import { useApiClient } from '../../contexts/ApiClientContext';
import { useUptime } from '../../hooks/useUptime';

export default function HomePage() {
	const apiClient = useApiClient();

	const { data } = useQuery({
		queryKey: ['live-stats'],
		queryFn: async () => {
			return apiClient.stats.live();
		},
		refetchInterval: 5 * 1000,
	});

	const uptime = useUptime(data?.live_since);

	return (
		<Box>
			<Heading size="lg" mb="2">
				Dashboard
			</Heading>
			<Text mb="8">Real-time DNS statistics and system status</Text>

			<Box
				display="grid"
				gridTemplateColumns={{
					base: '1fr',
					md: 'repeat(2, 1fr)',
					lg: 'repeat(5, 1fr)',
				}}
				gap="6"
				mb="8"
			>
				<StatCard
					label="Total Queries"
					value={data?.total ?? 0}
					icon={Globe}
					color="blue"
				/>
				<StatCard
					label="Queries Blocked"
					value={data?.blocked ?? 0}
					icon={Ban}
					color="red"
				/>
				<StatCard
					label="Total Errors"
					value={data?.errors ?? 0}
					icon={AlertCircle}
					color="red"
				/>
				<StatCard
					label="Queries cached"
					value={data?.cached ?? 0}
					icon={DatabaseBackup}
					color="red"
				/>
				<StatCard
					label="Avg Response"
					value={`${(data?.sum_duration && data.total ? data?.sum_duration / data?.total : 0).toFixed()} ms`}
					icon={Clock}
					color="yellow"
				/>
			</Box>
			<Box
				display="grid"
				gridTemplateColumns={{ base: '1fr', md: 'repeat(3, 1fr)' }}
				gap="6"
				mb="8"
			>
				<Box
					bg="gray.900"
					borderRadius="lg"
					borderWidth="1px"
					borderColor="gray.800"
					p="6"
				>
					<Text color="gray.400" fontSize="sm" mb="2">
						System Uptime
					</Text>
					<HStack align="baseline" gap="2">
						<Text color="white" fontSize="2xl" fontWeight="bold">
							{uptime.text}
						</Text>
					</HStack>
					<HStack mt="4" gap="2">
						<Box w="3" h="3" borderRadius="full" bg="green.500" />
						<Text color="green.400" fontSize="sm">
							System Online
						</Text>
					</HStack>
				</Box>
			</Box>
			<RecentActivity />
		</Box>
	);
}
