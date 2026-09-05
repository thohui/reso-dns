import {
	Box,
	CloseButton,
	Drawer,
	HStack,
	Icon,
	Portal,
	Spinner,
	Text,
	VStack,
} from '@chakra-ui/react';
import { Ban, ShieldCheck } from 'lucide-react';
import { useDomainRuleDetails } from '@/hooks/domain-rules/useDomainRuleDetails';
import type { Activity } from '@/lib/api/activity';
import type { DomainRule, ListAction, MatchType } from '@/lib/api/domain-rules';
import { getStatusInfo } from '@/lib/status-info';
import { formatTimeAgo } from '@/lib/time';

const MATCH_TYPE_LABELS: Record<MatchType, string> = {
	domain: 'Domain',
	wildcard: 'Wildcard',
	exact: 'Exact',
};

const ACTION_LABELS: Record<ListAction, string> = {
	block: 'Block',
	allow: 'Allow',
};

function DetailRow({ label, value }: { label: string; value: string }) {
	return (
		<HStack justify='space-between' py='2'>
			<Text
				fontSize='xs'
				color='fg.faint'
				textTransform='uppercase'
				whiteSpace='nowrap'
				overflow='hidden'
				textOverflow='ellipsis'
			>
				{label}
			</Text>
			<Text
				fontSize='sm'
				fontFamily="'Mozilla Text', sans-serif"
				whiteSpace='nowrap'
				overflow='hidden'
				textOverflow='ellipsis'
				color='fg'
			>
				{value}
			</Text>
		</HStack>
	);
}

function Section({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) {
	return (
		<Box>
			<Text
				fontSize='xs'
				fontWeight='600'
				color='fg.muted'
				textTransform='uppercase'
				letterSpacing='0.05em'
				mb='2'
			>
				{title}
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
				{children}
			</VStack>
		</Box>
	);
}

function Stat({ label, value }: { label: string; value: string }) {
	return (
		<VStack align='start' gap='0.5' flex='1' minW='0'>
			<Text fontSize='xs' color='fg.faint' textTransform='uppercase'>
				{label}
			</Text>
			<Text
				fontSize='lg'
				fontWeight='600'
				color='fg'
				fontFamily="'Mozilla Text', sans-serif"
				whiteSpace='nowrap'
				overflow='hidden'
				textOverflow='ellipsis'
			>
				{value}
			</Text>
		</VStack>
	);
}

function ActivityRow({ activity }: { activity: Activity }) {
	const statusInfo = getStatusInfo(activity);

	return (
		<HStack justify='space-between' gap='3' py='2.5'>
			<HStack gap='2.5' minW='0'>
				<Icon
					as={statusInfo.icon}
					boxSize='3.5'
					color={statusInfo.color}
					flexShrink='0'
				/>
				<VStack align='start' gap='0' minW='0'>
					<Text
						fontSize='sm'
						fontFamily="'Mozilla Text', sans-serif"
						color='fg'
						whiteSpace='nowrap'
						overflow='hidden'
						textOverflow='ellipsis'
					>
						{activity.qname || '-'}
					</Text>
					<Text fontSize='xs' color='fg.faint'>
						{activity.client || 'unknown'}
					</Text>
				</VStack>
			</HStack>
			<Text fontSize='xs' color='fg.muted' whiteSpace='nowrap'>
				{formatTimeAgo(activity.timestamp)}
			</Text>
		</HStack>
	);
}

interface DomainRuleDetailDrawerProps {
	rule: DomainRule | null;
	open: boolean;
	onClose: () => void;
}

export function DomainRuleDetailDrawer({
	rule,
	open,
	onClose,
}: DomainRuleDetailDrawerProps) {
	const { data, isLoading, isError } = useDomainRuleDetails(
		open && rule ? rule.id : null,
	);

	if (!rule) return null;

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
								<HStack gap='3' minW='0'>
									<Icon
										as={rule.action === 'block' ? Ban : ShieldCheck}
										boxSize='4'
										color={
											rule.action === 'block' ? 'status.blocked' : 'status.ok'
										}
									/>
									<Text
										fontWeight='500'
										fontSize='sm'
										whiteSpace='nowrap'
										overflow='hidden'
										textOverflow='ellipsis'
									>
										Rule Detail
									</Text>
								</HStack>
								<CloseButton size='sm' onClick={onClose} />
							</HStack>
						</Drawer.Header>

						<Drawer.Body px='5' py='4'>
							<VStack align='stretch' gap='5'>
								<Section title='Rule'>
									<DetailRow label='Domain' value={rule.domain} />
									<DetailRow
										label='Match'
										value={MATCH_TYPE_LABELS[rule.match_type]}
									/>
									<DetailRow
										label='Action'
										value={ACTION_LABELS[rule.action]}
									/>
									{rule.subscription_id !== null && (
										<DetailRow
											label='Subscription'
											value={data?.subscription_name ?? '-'}
										/>
									)}
									<DetailRow
										label='Status'
										value={rule.enabled ? 'Enabled' : 'Disabled'}
									/>
									<DetailRow
										label='Added'
										value={formatTimeAgo(rule.created_at)}
									/>
								</Section>

								<Box>
									<Text
										fontSize='xs'
										fontWeight='600'
										color='fg.muted'
										textTransform='uppercase'
										letterSpacing='0.05em'
										mb='2'
									>
										Activity
									</Text>
									<Box
										bg='bg.subtle'
										borderRadius='lg'
										px='4'
										py='3'
										borderWidth='1px'
										borderColor='border'
									>
										{isLoading ? (
											<HStack justify='center' py='2'>
												<Spinner size='sm' color='fg.muted' />
											</HStack>
										) : isError || !data ? (
											<Text fontSize='sm' color='fg.muted'>
												Could not load activity
											</Text>
										) : (
											<HStack gap='4' align='start'>
												<Stat
													label='Blocked'
													value={data.total_blocked.toLocaleString()}
												/>
												<Stat
													label='Allowed'
													value={data.total_allowed.toLocaleString()}
												/>
												<Stat
													label='Last seen'
													value={formatTimeAgo(data.last_seen_at)}
												/>
											</HStack>
										)}
									</Box>
								</Box>

								{data && data.activities.length > 0 && (
									<Section title='Recent activity'>
										{data.activities.map((activity) => (
											<ActivityRow
												key={`${activity.kind}-${activity.d.source_id}`}
												activity={activity}
											/>
										))}
									</Section>
								)}

								{data && data.activities.length === 0 && (
									<Section title='Recent activity'>
										<Text fontSize='sm' color='fg.muted' py='2'>
											No queries have matched this rule yet
										</Text>
									</Section>
								)}
							</VStack>
						</Drawer.Body>
					</Drawer.Content>
				</Drawer.Positioner>
			</Portal>
		</Drawer.Root>
	);
}
