import {
	Box,
	Button,
	CloseButton,
	Drawer,
	HStack,
	Icon,
	Portal,
	Text,
	VStack,
} from '@chakra-ui/react';
import { Ban } from 'lucide-react';
import { useBlockDomain } from '../../hooks/useBlockDomain';
import {
	type Activity,
	type ErrorActivity,
	getErrorTypeLabel,
	getResponseCodeLabel,
	getTransportLabel,
	type QueryActivity,
} from '../../lib/api/activity';
import { recordTypeName } from '../../lib/dns';
import { getStatusInfo } from '../../lib/status-info';
import { toastError } from '../Toaster';

function DetailRow({ label, value }: { label: string; value: string }) {
	return (
		<HStack justify='space-between' py='2'>
			<Text fontSize='xs' color='fg.faint' textTransform='uppercase'>
				{label}
			</Text>
			<Text fontSize='sm' fontFamily="'Mozilla Text', sans-serif" color='fg'>
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
	const blockDomain = useBlockDomain();

	if (!activity) return null;

	const statusInfo = getStatusInfo(activity);
	const canBlock =
		activity.kind === 'query' && activity.qname && !activity.d.blocked;

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
		<Drawer.Root
			open={open}
			onOpenChange={(e) => !e.open && onClose()}
			placement='end'
			size='sm'
		>
			<Portal>
				<Drawer.Backdrop />
				<Drawer.Positioner>
					<Drawer.Content
						bg='bg.panel'
						borderColor='border'
						borderLeftWidth='1px'
					>
						<Drawer.Header
							borderBottomWidth='1px'
							borderColor='border'
							px='5'
							py='4'
						>
							<HStack justify='space-between' w='full'>
								<HStack gap='3'>
									<Icon
										as={statusInfo.icon}
										boxSize='4'
										color={statusInfo.color}
									/>
									<Text fontWeight='500' fontSize='sm'>
										{activity.kind === 'query'
											? 'Query Detail'
											: 'Error Detail'}
									</Text>
								</HStack>
								<CloseButton size='sm' onClick={onClose} />
							</HStack>
						</Drawer.Header>

						<Drawer.Body px='5' py='4'>
							<VStack align='stretch' gap='5'>
								<HStack justify='space-between'>
									<Text
										fontSize='xs'
										color='fg.faint'
										textTransform='uppercase'
									>
										Status
									</Text>
									<StyledBadge bg={statusInfo.bg} color={statusInfo.color}>
										{statusInfo.label}
									</StyledBadge>
								</HStack>

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
										<DetailRow label='Domain' value={activity.qname || '-'} />
										<DetailRow
											label='Type'
											value={
												activity.qtype !== null
													? recordTypeName(activity.qtype)
													: '-'
											}
										/>
										<DetailRow
											label='Client'
											value={activity.client || 'unknown'}
										/>
										<DetailRow
											label='Transport'
											value={getTransportLabel(activity.transport)}
										/>
										<DetailRow label='Time' value={time} />
										<DetailRow label='Duration' value={durationStr} />
									</VStack>
								</Box>

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
												value={getResponseCodeLabel(activity.d.rcode)}
											/>
											<DetailRow
												label='Cache Hit'
												value={
													(activity as QueryActivity).d.cache_hit ? 'Yes' : 'No'
												}
											/>
											<DetailRow
												label='Blocked'
												value={
													(activity as QueryActivity).d.blocked ? 'Yes' : 'No'
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
												value={getErrorTypeLabel(
													(activity as ErrorActivity).d.error_type,
												)}
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

						{canBlock && (
							<Drawer.Footer
								borderTopWidth='1px'
								borderColor='border'
								px='5'
								py='4'
							>
								<Button
									w='full'
									variant='ghost'
									borderWidth='1px'
									borderColor='border'
									color='fg.muted'
									_hover={{
										bg: 'status.error/10',
										borderColor: 'status.error',
										color: 'status.error',
									}}
									onClick={() => {
										blockDomain.mutate(activity.qname!, {
											onError: (e) => toastError(e),
											onSuccess: () => onClose(),
										});
									}}
									loading={blockDomain.isPending}
								>
									<Icon as={Ban} boxSize='3.5' mr='2' />
									Block {activity.qname}
								</Button>
							</Drawer.Footer>
						)}
					</Drawer.Content>
				</Drawer.Positioner>
			</Portal>
		</Drawer.Root>
	);
}
