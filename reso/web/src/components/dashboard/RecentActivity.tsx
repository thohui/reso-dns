import { Box, HStack, Icon, Text, VStack } from '@chakra-ui/react';
import { Zap } from 'lucide-react';
import { useRecentActivity } from '@/hooks/dashboard/useRecentActivity';
import { type Activity, getTransportLabel } from '@/lib/api/activity';
import { getStatusInfo } from '@/lib/status-info';

export function RecentActivity() {
	const activities = useRecentActivity();

	return (
		<Box
			bg='bg.panel'
			borderRadius='xl'
			borderWidth='1px'
			borderColor='border'
			overflow='hidden'
		>
			<HStack
				justify='space-between'
				px='5'
				py='3'
				borderBottomWidth='1px'
				borderColor='border'
			>
				<HStack gap='2'>
					<Icon as={Zap} boxSize='4' color='accent.subtle' />
					<Text fontWeight='500' fontSize='sm'>
						Recent Activity
					</Text>
				</HStack>
			</HStack>
			<VStack gap='0' align='stretch' overflowX='auto'>
				{activities?.items.slice(0, 5).map((activity, i) => (
					<ActivityRow key={`${activity.timestamp}-${i}`} activity={activity} />
				))}
			</VStack>
		</Box>
	);
}

function ActivityRow({ activity }: { activity: Activity }) {
	const status = getStatusInfo(activity);

	const time = new Date(activity.timestamp).toLocaleTimeString('en-US', {
		hour12: false,
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
	});

	return (
		<HStack
			justify='space-between'
			align='center'
			px='5'
			py='3'
			borderBottomWidth='1px'
			borderColor='border'
			_last={{ borderBottom: 'none' }}
			_hover={{ bg: 'bg.subtle' }}
			transition='background 0.1s ease'
		>
			<HStack gap='3' align='center'>
				<Icon as={status.icon} boxSize='4' color={status.color} />
				<Text
					fontFamily="'Mozilla Text', sans-serif"
					fontSize='sm'
					fontWeight='500'
					lineHeight='1'
				>
					{activity.qname || '-'}
				</Text>
			</HStack>
			<HStack gap='3' align='center'>
				<Box
					px='2'
					py='0.5'
					borderRadius='md'
					fontSize='sm'
					fontWeight='500'
					fontFamily="'Mozilla Text', sans-serif"
					bg='accent.muted'
					color='accent.fg'
				>
					{getTransportLabel(activity.transport)}
				</Box>
				<Text
					color='fg.faint'
					fontSize='sm'
					fontFamily="'Mozilla Text', sans-serif"
					lineHeight='1'
				>
					{time}
				</Text>
			</HStack>
		</HStack>
	);
}
