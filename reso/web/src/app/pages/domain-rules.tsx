import { AddRuleDialog } from '@/components/domain-rules/AddRuleDialog';
import { DomainRulesGrid } from '@/components/domain-rules/DomainRulesGrid';
import { EditRuleDialog } from '@/components/domain-rules/EditRuleDialog';
import { SubscriptionDialog } from '@/components/domain-rules/SubscriptionDialog';
import { SubscriptionsGrid } from '@/components/domain-rules/SubscriptionsGrid';
import { toastError } from '@/components/Toaster';
import { useAddDomainRule } from '@/hooks/domain-rules/useAddDomainRule';
import { useAddSubscription } from '@/hooks/domain-rules/useAddSubscription';
import {
	DOMAIN_RULES_PAGE_SIZE,
	domainRulesQueryKey,
	useDomainRules,
} from '@/hooks/domain-rules/useDomainRules';
import {
	listSubscriptionsQueryKey,
	useListSubscriptions,
} from '@/hooks/domain-rules/useListSubscriptions';
import { useRemoveDomainRule } from '@/hooks/domain-rules/useRemoveDomainRule';
import { useRemoveSubscription } from '@/hooks/domain-rules/useRemoveSubscription';
import { useToggleDomainRule } from '@/hooks/domain-rules/useToggleDomainRule';
import { useToggleSubscription } from '@/hooks/domain-rules/useToggleSubscription';
import { useToggleSubscriptionSync } from '@/hooks/domain-rules/useToggleSubscriptionSync';
import { useUpdateDomainRule } from '@/hooks/domain-rules/useUpdateDomainRule';
import { useDebounce } from '@/hooks/useDebounce';
import type { DomainRule, ListAction, MatchType } from '@/lib/api/domain-rules';
import type { ListSubscription } from '@/lib/api/list-subscriptions';
import type { PagedResponse } from '@/lib/api/pagination';
import { Box, Button, Icon, Tabs, Text } from '@chakra-ui/react';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, List, Plus } from 'lucide-react';
import { useCallback, useState } from 'react';

export default function DomainRulesPage() {
	const queryClient = useQueryClient();

	const [page, setPage] = useState(0);
	const [search, setSearch] = useState('');

	const debouncedSearch = useDebounce(search, 300);

	const { data: rulesData, isFetching: rulesFetching } = useDomainRules(
		page,
		debouncedSearch,
	);
	const { data: subscriptions } = useListSubscriptions();

	const invalidateRules = useCallback(
		() => queryClient.invalidateQueries({ queryKey: ['domain-rules'] }),
		[queryClient],
	);
	const invalidateSubs = useCallback(
		() =>
			queryClient.invalidateQueries({ queryKey: listSubscriptionsQueryKey }),
		[queryClient],
	);

	const [showRuleDialog, setShowRuleDialog] = useState(false);
	const [showSubscriptionDialog, setShowSubscriptionDialog] = useState(false);
	const [editingRule, setEditingRule] = useState<DomainRule | null>(null);

	const addRule = useAddDomainRule();
	const { mutateAsync: removeRule } = useRemoveDomainRule();
	const { mutateAsync: toggleRule } = useToggleDomainRule();
	const updateRule = useUpdateDomainRule();
	const addSubscription = useAddSubscription();
	const { mutateAsync: removeSubscription } = useRemoveSubscription();
	const { mutateAsync: toggleSubscription } = useToggleSubscription();
	const { mutateAsync: toggleSubscriptionSync } = useToggleSubscriptionSync();

	const total = rulesData?.total;
	const totalPages =
		total != null
			? Math.max(1, Math.ceil(total / DOMAIN_RULES_PAGE_SIZE))
			: null;

	const handleSearchChange = (value: string) => {
		setSearch(value);
		setPage(0);
	};

	const handleAddRule = async (
		domain: string,
		matchType: MatchType,
		action: ListAction,
	) => {
		await addRule.mutateAsync({ domain, matchType, action });
		invalidateRules();
		setShowRuleDialog(false);
	};

	const handleRemoveRule = useCallback(
		async (domain: string) => {
			try {
				await removeRule(domain);
			} catch (e) {
				toastError(e);
				return;
			}
			invalidateRules();
			setEditingRule((current) =>
				current?.domain === domain ? null : current,
			);
		},
		[removeRule, invalidateRules],
	);

	const handleToggleRule = useCallback(
		async (domain: string) => {
			const previous = queryClient.getQueryData<PagedResponse<DomainRule>>(
				domainRulesQueryKey(page, debouncedSearch),
			);
			queryClient.setQueryData<PagedResponse<DomainRule>>(
				domainRulesQueryKey(page, debouncedSearch),
				(old) => {
					if (!old) return old;
					return {
						...old,
						items: old.items.map((d) =>
							d.domain === domain ? { ...d, enabled: !d.enabled } : d,
						),
					};
				},
			);

			try {
				await toggleRule(domain);
			} catch (e) {
				queryClient.setQueryData(
					domainRulesQueryKey(page, debouncedSearch),
					previous,
				);
				toastError(e);
			}
		},
		[queryClient, toggleRule, page, debouncedSearch],
	);

	const handleEditRule = async (domain: string, action: ListAction) => {
		await updateRule.mutateAsync({ domain, action });
		invalidateRules();
		setEditingRule((current) => (current?.domain === domain ? null : current));
	};

	const handleAddSubscription = async (
		name: string,
		url: string,
		list_type: ListAction,
		sync_enabled: boolean,
	) => {
		await addSubscription.mutateAsync({ name, url, list_type, sync_enabled });
		invalidateRules();
		invalidateSubs();
		setShowSubscriptionDialog(false);
	};

	const handleRemoveSubscription = useCallback(
		async (id: string) => {
			try {
				await removeSubscription(id);
			} catch (e) {
				toastError(e);
				return;
			}
			invalidateRules();
			invalidateSubs();
		},
		[removeSubscription, invalidateRules, invalidateSubs],
	);

	const handleToggleSubscription = useCallback(
		async (id: string) => {
			const previous = queryClient.getQueryData<ListSubscription[]>([
				'list-subscriptions',
			]);
			queryClient.setQueryData<ListSubscription[]>(
				['list-subscriptions'],
				(old) =>
					old?.map((s) => (s.id === id ? { ...s, enabled: !s.enabled } : s)),
			);
			try {
				await toggleSubscription(id);
			} catch (e) {
				queryClient.setQueryData(['list-subscriptions'], previous);
				toastError(e);
			}
		},
		[queryClient, toggleSubscription],
	);

	const handleToggleSubscriptionSync = useCallback(
		async (id: string) => {
			const previous = queryClient.getQueryData<ListSubscription[]>([
				'list-subscriptions',
			]);
			queryClient.setQueryData<ListSubscription[]>(
				['list-subscriptions'],
				(old) =>
					old?.map((s) =>
						s.id === id ? { ...s, sync_enabled: !s.sync_enabled } : s,
					),
			);
			try {
				await toggleSubscriptionSync(id);
			} catch (e) {
				queryClient.setQueryData(['list-subscriptions'], previous);
				toastError(e);
			}
		},
		[queryClient, toggleSubscriptionSync],
	);

	const rules = rulesData?.items ?? [];

	return (
		<Box display='flex' flexDir='column' flex='1' minH='0'>
			<Tabs.Root
				defaultValue='rules'
				variant='line'
				display='flex'
				flexDir='column'
				flex='1'
				minH='0'
			>
				<Box
					display='flex'
					flexDir={{ base: 'column', sm: 'row' }}
					justifyContent='space-between'
					alignItems={{ base: 'flex-start', sm: 'center' }}
					gap={{ base: '3', sm: '0' }}
					mb='4'
					w='full'
					flexShrink='0'
				>
					<Tabs.List borderColor='border'>
						<Tabs.Trigger
							value='rules'
							color='fg.muted'
							_hover={{ color: 'fg' }}
							_selected={{ color: 'accent.fg', _hover: { color: 'accent.fg' } }}
						>
							<Icon as={Ban} boxSize='3.5' mr='2' />
							Rules
							{total != null && (
								<Text as='span' ml='2' fontSize='xs' color='fg.subtle'>
									{total}
								</Text>
							)}
						</Tabs.Trigger>
						<Tabs.Trigger
							value='subscriptions'
							color='fg.muted'
							_hover={{ color: 'fg' }}
							_selected={{ color: 'accent.fg', _hover: { color: 'accent.fg' } }}
						>
							<Icon as={List} boxSize='3.5' mr='2' />
							Subscriptions
							<Text as='span' ml='2' fontSize='xs' color='fg.subtle'>
								{subscriptions?.length ?? 0}
							</Text>
						</Tabs.Trigger>
					</Tabs.List>
					<Tabs.Context>
						{({ value }) => (
							<Box>
								{value === 'rules' ? (
									<Button
										bg='accent'
										color='fg'
										_hover={{ bg: 'accent.hover' }}
										h='9'
										fontSize='sm'
										onClick={() => setShowRuleDialog(true)}
									>
										<Icon as={Plus} boxSize='3.5' mr='2' />
										Add Rule
									</Button>
								) : (
									<Button
										bg='accent'
										color='fg'
										_hover={{ bg: 'accent.hover' }}
										h='9'
										fontSize='sm'
										onClick={() => setShowSubscriptionDialog(true)}
									>
										<Icon as={Plus} boxSize='3.5' mr='2' />
										Add Subscription
									</Button>
								)}
							</Box>
						)}
					</Tabs.Context>
				</Box>
				<Tabs.Content
					value='rules'
					pt='0'
					flex='1'
					minH='0'
					overflow='hidden'
					display='flex'
					flexDir='column'
				>
					<DomainRulesGrid
						rules={rules}
						page={page}
						totalPages={totalPages}
						total={total}
						onPageChange={setPage}
						search={search}
						onSearchChange={handleSearchChange}
						onRemove={handleRemoveRule}
						onToggle={handleToggleRule}
						onEdit={setEditingRule}
						isLoading={rulesFetching}
					/>
				</Tabs.Content>

				<Tabs.Content
					value='subscriptions'
					pt='0'
					flex='1'
					minH='0'
					overflow='hidden'
					display='flex'
					flexDir='column'
				>
					<SubscriptionsGrid
						subscriptions={subscriptions ?? []}
						onRemove={handleRemoveSubscription}
						onToggle={handleToggleSubscription}
						onToggleSync={handleToggleSubscriptionSync}
					/>
				</Tabs.Content>
			</Tabs.Root>

			{showRuleDialog && (
				<AddRuleDialog
					onClose={() => setShowRuleDialog(false)}
					onSubmit={handleAddRule}
				/>
			)}
			{showSubscriptionDialog && (
				<SubscriptionDialog
					onClose={() => setShowSubscriptionDialog(false)}
					onSubmit={handleAddSubscription}
				/>
			)}
			{editingRule && (
				<EditRuleDialog
					rule={editingRule}
					onClose={() => setEditingRule(null)}
					onSubmit={handleEditRule}
				/>
			)}
		</Box>
	);
}
