import {
	Box,
	CloseButton,
	Drawer,
	HStack,
	Icon,
	Portal,
	Text,
	VStack,
} from '@chakra-ui/react';
import {
	AlertTriangle,
	CheckCircle,
	ShieldOff,
	XCircle,
	Zap
} from 'lucide-react';
import {
	type Activity,
	ERROR_TYPE_LABELS,
	type ErrorActivity,
	type QueryActivity,
	RCODE_LABELS,
	RECORD_TYPES,
	TRANSPORT_LABELS,
} from '../../lib/api/activity';

const STATUS_BG: Record<string, string> = {
	error: 'status.errorMuted',
	blocked: 'status.blockedMuted',
	cached: 'status.cachedMuted',
	warn: 'status.warnMuted',
	success: 'status.successMuted',
};

const STATUS_FG: Record<string, string> = {
	error: 'status.error',
	blocked: 'status.blocked',
	cached: 'status.cached',
	warn: 'status.warn',
	success: 'status.success',
};

function getStatusKey(activity: Activity): string {
	if (activity.kind === 'error') return 'error';
	const q = activity as QueryActivity;
	if (q.d.blocked) return 'blocked';
	if (q.d.cache_hit) return 'cached';
	if (q.d.rcode !== 0) return 'warn';
	return 'success';
}

function getStatusLabel(activity: Activity): string {
	if (activity.kind === 'error') return 'ERROR';
	const q = activity as QueryActivity;
	if (q.d.blocked) return 'BLOCKED';
	if (q.d.cache_hit) return 'CACHED';
	if (q.d.rcode !== 0) return RCODE_LABELS[q.d.rcode] || `RCODE:${q.d.rcode}`;
	return 'OK';
}

function getStatusIcon(activity: Activity) {
	if (activity.kind === 'error') return XCircle;
	const q = activity as QueryActivity;
	if (q.d.blocked) return ShieldOff;
	if (q.d.cache_hit) return Zap;
	if (q.d.rcode !== 0) return AlertTriangle;
	return CheckCircle;
}

function DetailRow({ label, value }: { label: string; value: string; }) {
	return (
		<HStack justify='space-between' py='2'>
			<Text fontSize='xs' color='fg.faint' textTransform='uppercase'>
				{label}
			</Text>
			<Text
				fontSize='sm'
				fontFamily="'JetBrains Mono', monospace"
				color='fg'
			>
				{value}
			</Text>
		</HStack>
	);
}

function StyledBadge({
	bg,
	color,
	children,
}: {
	bg: string;
	color: string;
	children: React.ReactNode;
}) {
	return (
		<Box
			px='2.5'
			py='0.5'
			borderRadius='md'
			fontSize='xs'
			fontWeight='600'
			textTransform='uppercase'
			letterSpacing='0.03em'
			bg={bg}
			color={color}
		>
			{children}
		</Box>
	);
}

interface ActivityDetailDrawerProps {
	activity: Activity | null;
	open: boolean;
	onClose: () => void;
}

export function ActivityDetailDrawer({
	activity,
	open,
	onClose,
}: ActivityDetailDrawerProps) {
	if (!activity) return null;

	const statusKey = getStatusKey(activity);
	const statusLabel = getStatusLabel(activity);
	const statusIcon = getStatusIcon(activity);
	const time = new Date(activity.timestamp).toLocaleString('en-US', {
		hour12: false,
		year: 'numeric',
		month: '2-digit',
		day: '2-digit',
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
	});

	const durationStr =
		activity.duration >= 1000
			? `${(activity.duration / 1000).toFixed(1)}s`
			: `${activity.duration}ms`;

	return (
		<Drawer.Root open={open} onOpenChange={(e) => !e.open && onClose()} placement='end' size='sm'>
			<Portal>
				<Drawer.Backdrop />
				<Drawer.Positioner>
					<Drawer.Content bg='bg.panel' borderColor='border' borderLeftWidth='1px'>
						<Drawer.Header
							borderBottomWidth='1px'
							borderColor='border'
							px='5'
							py='4'
						>
							<HStack justify='space-between' w='full'>
								<HStack gap='3'>
									<Icon
										as={statusIcon}
										boxSize='4'
										color={STATUS_FG[statusKey]}
									/>
									<Text fontWeight='500' fontSize='sm'>
										Query Detail
									</Text>
								</HStack>
								<CloseButton size='sm' onClick={onClose} />
							</HStack>
						</Drawer.Header>

						<Drawer.Body px='5' py='4'>
							<VStack align='stretch' gap='5'>
								{/* Status */}
								<HStack justify='space-between'>
									<Text fontSize='xs' color='fg.faint' textTransform='uppercase'>
										Status
									</Text>
									<StyledBadge
										bg={STATUS_BG[statusKey]}
										color={STATUS_FG[statusKey]}
									>
										{statusLabel}
									</StyledBadge>
								</HStack>

								{/* General */}
								<Box>
									<Text
										fontSize='xs'
										fontWeight='600'
										color='fg.muted'
										textTransform='uppercase'
										letterSpacing='0.05em'
										mb='2'
									>
										General
									</Text>
									<VStack
										align='stretch'
										gap='0'
										bg='bg.subtle'
										borderRadius='lg'
										px='4'
										py='1'
										borderWidth='1px'
										borderColor='border'
									>
										<DetailRow
											label='Domain'
											value={activity.qname || '-'}
										/>
										<DetailRow
											label='Type'
											value={
												activity.qtype !== null
													? RECORD_TYPES[activity.qtype] || `${activity.qtype}`
													: '-'
											}
										/>
										<DetailRow
											label='Client'
											value={activity.client || 'unknown'}
										/>
										<DetailRow
											label='Transport'
											value={
												TRANSPORT_LABELS[activity.transport] ||
												`T:${activity.transport}`
											}
										/>
										<DetailRow label='Time' value={time} />
										<DetailRow label='Duration' value={durationStr} />
									</VStack>
								</Box>

								{/* Resolution / Error details */}
								{activity.kind === 'query' && (
									<Box>
										<Text
											fontSize='xs'
											fontWeight='600'
											color='fg.muted'
											textTransform='uppercase'
											letterSpacing='0.05em'
											mb='2'
										>
											Resolution
										</Text>
										<VStack
											align='stretch'
											gap='0'
											bg='bg.subtle'
											borderRadius='lg'
											px='4'
											py='1'
											borderWidth='1px'
											borderColor='border'
										>
											<DetailRow
												label='Response Code'
												value={
													RCODE_LABELS[(activity as QueryActivity).d.rcode] ||
													`${(activity as QueryActivity).d.rcode}`
												}
											/>
											<DetailRow
												label='Cache Hit'
												value={
													(activity as QueryActivity).d.cache_hit
														? 'Yes'
														: 'No'
												}
											/>
											<DetailRow
												label='Blocked'
												value={
													(activity as QueryActivity).d.blocked
														? 'Yes'
														: 'No'
												}
											/>
										</VStack>
									</Box>
								)}

								{activity.kind === 'error' && (
									<Box>
										<Text
											fontSize='xs'
											fontWeight='600'
											color='fg.muted'
											textTransform='uppercase'
											letterSpacing='0.05em'
											mb='2'
										>
											Error
										</Text>
										<VStack
											align='stretch'
											gap='0'
											bg='bg.subtle'
											borderRadius='lg'
											px='4'
											py='1'
											borderWidth='1px'
											borderColor='border'
										>
											<DetailRow
												label='Error Type'
												value={
													ERROR_TYPE_LABELS[
													(activity as ErrorActivity).d.error_type
													] || `Type ${(activity as ErrorActivity).d.error_type}`
												}
											/>
											<DetailRow
												label='Message'
												value={(activity as ErrorActivity).d.message}
											/>
										</VStack>
									</Box>
								)}
							</VStack>
						</Drawer.Body>
					</Drawer.Content>
				</Drawer.Positioner>
			</Portal>
		</Drawer.Root>
	);
}
