import { Box, HStack, Icon, Text, VStack } from '@chakra-ui/react';
import {
	AlertTriangle,
	CheckCircle,
	type LucideIcon,
	ShieldOff,
	XCircle,
	Zap,
} from 'lucide-react';
import { useRecentActivity } from '../../hooks/useRecentActivity';
import {
	type Activity,
	getTransportLabel,
	type QueryActivity,
} from '../../lib/api/activity';

interface StatusInfo {
	label: string;
	color: string;
	bg: string;
	icon: LucideIcon;
}

function getStatusInfo(activity: Activity): StatusInfo {
	if (activity.kind === 'error') {
		return {
			label: 'error',
			color: 'status.error',
			bg: 'status.errorMuted',
			icon: XCircle,
		};
	}

	const q = activity as QueryActivity;

	if (q.d.blocked) {
		return {
			label: 'blocked',
			color: 'status.blocked',
			bg: 'status.blockedMuted',
			icon: ShieldOff,
		};
	}

	if (q.d.rcode !== 0) {
		return {
			label: 'warning',
			color: 'status.warn',
			bg: 'status.warnMuted',
			icon: AlertTriangle,
		};
	}

	if (q.d.cache_hit) {
		return {
			label: 'cached',
			color: 'status.cached',
			bg: 'status.cachedMuted',
			icon: Zap,
		};
	}

	return {
		label: 'ok',
		color: 'status.success',
		bg: 'status.successMuted',
		icon: CheckCircle,
	};
}

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
				py='4'
				borderBottomWidth='1px'
				borderColor='border'
			>
				<HStack gap='2'>
					<Icon as={Zap} boxSize='4' color='accent.subtle' />
					<Text fontWeight='500' fontSize='sm'>
						Recent Activity
					</Text>
				</HStack>
				<Text fontSize='xs' color='fg.faint'>
					Last 5 entries
				</Text>
			</HStack>
			<VStack gap='0' align='stretch'>
				{activities?.items.map((activity, i) => (
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
			px='5'
			py='3'
			borderBottomWidth='1px'
			borderColor='border'
			_last={{ borderBottom: 'none' }}
			_hover={{ bg: 'bg.subtle' }}
			transition='background 0.1s ease'
		>
			<HStack gap='3'>
				<Icon as={status.icon} boxSize='3.5' color={status.color} />
				<Text
					fontFamily="'JetBrains Mono', monospace"
					fontSize='xs'
					fontWeight='500'
				>
					{activity.qname || '-'}
				</Text>
				<Box
					px='2.5'
					py='0.5'
					borderRadius='md'
					fontSize='xs'
					fontWeight='600'
					textTransform='uppercase'
					letterSpacing='0.03em'
					bg={status.bg}
					color={status.color}
				>
					{status.label}
				</Box>
			</HStack>
			<HStack gap='3'>
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
					{getTransportLabel(activity.transport)}
				</Box>
				<Text
					color='fg.faint'
					fontSize='xs'
					fontFamily="'JetBrains Mono', monospace"
				>
					{time}
				</Text>
			</HStack>
		</HStack>
	);
}
