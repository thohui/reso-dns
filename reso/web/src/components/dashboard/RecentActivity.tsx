import {
	Badge,
	Box,
	Heading,
	HStack,
	Icon,
	Text,
	VStack,
} from '@chakra-ui/react';
import { CheckCircle, ShieldOff, XCircle } from 'lucide-react';
import { useRecentActivity } from '../../hooks/useRecentActivity';
import {
	type Activity,
	type QueryActivity,
	TRANSPORT_LABELS,
} from '../../lib/api/activity';

export function RecentActivity() {
	const activities = useRecentActivity();

	return (
		<Box
			bg='gray.900'
			borderRadius='lg'
			borderWidth='1px'
			borderColor='gray.800'
			p='6'
		>
			<Heading size='md' color='white' mb='4'>
				Recent Activity
			</Heading>
			<VStack gap='3' align='stretch'>
				{activities?.items.map((activity) => (
					<ActivityRow key={activity.timestamp} activity={activity} />
				))}
			</VStack>
		</Box>
	);
}

function ActivityRow({ activity }: { activity: Activity; }) {
	const isError = activity.kind === 'error';

	const isBlocked =
		activity.kind === 'query' && (activity as QueryActivity).d.blocked;

	let statusColor: string = 'status.success';
	let badgePalette: string = 'green';
	let statusLabel: string = 'ok';
	let icon = CheckCircle;

	if (isError) {
		statusColor = 'status.error';
		statusLabel = 'error';
		badgePalette = 'red';
		icon = XCircle;
	} else if (isBlocked) {
		statusColor = 'status.blocked';
		statusLabel = 'blocked';
		badgePalette = 'red';
		icon = ShieldOff;
	}

	const time = new Date(activity.timestamp).toLocaleTimeString('en-US', {
		hour12: false,
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
	});

	return (
		<HStack
			justify='space-between'
			py='2'
			borderBottomWidth='1px'
			borderColor='border'
			_last={{ border: 'none' }}
		>
			<HStack gap='3'>
				<Icon as={icon} boxSize='4' color={statusColor} />
				<Text fontFamily='mono' fontSize='sm'>
					{activity.qname || '-'}
				</Text>
				<Badge colorPalette={badgePalette} size='sm'>
					{statusLabel}
				</Badge>
			</HStack>
			<HStack gap='4'>
				<Badge colorPalette='gray' size='sm' variant='subtle'>
					{TRANSPORT_LABELS[activity.transport] || '?'}
				</Badge>
				<Text color='fg.subtle' fontSize='sm'>
					{activity.client || 'unknown'}
				</Text>
				<Text color='fg.faint' fontSize='sm' fontFamily='mono'>
					{time}
				</Text>
			</HStack>
		</HStack>
	);
}
