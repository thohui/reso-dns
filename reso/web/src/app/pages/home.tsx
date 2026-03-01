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
			<Heading size='lg' mb='2'>
				Dashboard
			</Heading>
			<Text mb='8'>Real-time DNS statistics and system status</Text>

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
					value={data?.total ?? 0}
					icon={Globe}
					accentColor='accent'
				/>
				<StatCard
					label='Queries Blocked'
					value={data?.blocked ?? 0}
					icon={Ban}
					accentColor='accent'
				/>
				<StatCard
					label='Total Errors'
					value={data?.errors ?? 0}
					icon={AlertCircle}
					accentColor='accent'
				/>
				<StatCard
					label='Queries cached'
					value={data?.cached ?? 0}
					icon={DatabaseBackup}
					accentColor='accent'
				/>
				<StatCard
					label='Avg Response'
					value={`${(data?.sum_duration && data.total ? data?.sum_duration / data?.total : 0).toFixed()} ms`}
					icon={Clock}
					accentColor='accent'
				/>
			</Box>
			<Box
				display='grid'
				gridTemplateColumns={{ base: '1fr', md: 'repeat(3, 1fr)' }}
				gap='6'
				mb='8'
			>
				<Box
					bg='bg.panel'
					borderRadius='xl'
					borderWidth='1px'
					borderColor='border'
					p='5'
				>
					<Text
						color='fg.subtle'
						fontSize='xs'
						fontWeight='500'
						textTransform='uppercase'
						letterSpacing='0.05em'
						mb='3'
					>
						System Uptime
					</Text>
					<Text fontSize='2xl' fontWeight='600' letterSpacing='-0.02em' mb='3'>
						{uptime.text}
					</Text>
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
						<Text color='status.success' fontSize='xs' fontWeight='500'>
							All systems operational
						</Text>
					</HStack>
				</Box>
			</Box>
			<RecentActivity />
		</Box>
	);
}
