import {
	Box,
	Button,
	HStack,
	Heading,
	Icon,
	Tabs,
	Text,
} from '@chakra-ui/react';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, List, Plus } from 'lucide-react';
import { useRef, useState } from 'react';
import { SubscriptionDialog } from '@/components/domain-rules/SubscriptionDialog';
import { SubscriptionsGrid } from '@/components/domain-rules/SubscriptionsGrid';
import { AddRuleDialog } from '@/components/domain-rules/AddRuleDialog';
import { DomainRulesGrid } from '@/components/domain-rules/DomainRulesGrid';
import { EditRuleDialog } from '@/components/domain-rules/EditRuleDialog';
import { toastError } from '@/components/Toaster';
import { useAddDomainRule } from '@/hooks/useAddDomainRule';
import { useDomainRules, DOMAIN_RULES_PAGE_SIZE } from '@/hooks/useDomainRules';
import { useListSubscriptions } from '@/hooks/useListSubscriptions';
import { useRemoveDomainRule } from '@/hooks/useRemoveDomainRule';
import { useRemoveSubscription } from '@/hooks/useRemoveSubscription';
import { useToggleDomainRule } from '@/hooks/useToggleDomainRule';
import { useToggleSubscription } from '@/hooks/useToggleSubscription';
import { useUpdateDomainRule } from '@/hooks/useUpdateDomainRule';
import { useAddSubscription } from '@/hooks/useAddSubscription';
import { useDebounce } from '@/hooks/useDebounce';
import type { DomainRule, ListAction } from '@/lib/api/domain-rules';
import type { ListSubscription } from '@/lib/api/list-subscriptions';
import type { PagedResponse } from '@/lib/api/pagination';
import { useToggleSubscriptionSync } from '@/hooks/useToggleSubscriptionSync';

export default function DomainRulesPage() {
	const queryClient = useQueryClient();

	const [page, setPage] = useState(0);
	const [search, setSearch] = useState('');
	const cachedTotal = useRef<number | null>(null);

	const debouncedSearch = useDebounce(search, 300);

	const { data: rulesData, isFetching: rulesFetching } = useDomainRules(
		page,
		debouncedSearch,
	);
	const { data: subscriptions } = useListSubscriptions();

	const invalidateRules = () =>
		queryClient.invalidateQueries({ queryKey: ['domain-rules'] });
	const invalidateSubs = () =>
		queryClient.invalidateQueries({ queryKey: ['list-subscriptions'] });

	const [showRuleDialog, setShowRuleDialog] = useState(false);
	const [showSubscriptionDialog, setShowSubscriptionDialog] = useState(false);
	const [editingRule, setEditingRule] = useState<DomainRule | null>(null);

	const addRule = useAddDomainRule();
	const removeRule = useRemoveDomainRule();
	const toggleRule = useToggleDomainRule();
	const updateRule = useUpdateDomainRule();
	const addSubscription = useAddSubscription();
	const removeSubscription = useRemoveSubscription();
	const toggleSubscription = useToggleSubscription();
	const toggleSubscriptionSync = useToggleSubscriptionSync();

	if (rulesData?.total != null) {
		cachedTotal.current = rulesData.total;
	}

	const total = rulesData?.total ?? cachedTotal.current;
	const totalPages =
		total != null
			? Math.max(1, Math.ceil(total / DOMAIN_RULES_PAGE_SIZE))
			: null;

	const handleSearchChange = (value: string) => {
		setSearch(value);
		setPage(0);
	};

	const handleAddRule = async (domain: string, action: ListAction) => {
		await addRule.mutateAsync({ domain, action }, { onError: toastError });
		invalidateRules();
		setShowRuleDialog(false);
	};

	const handleRemoveRule = async (domain: string) => {
		await removeRule.mutateAsync(domain, { onError: toastError });
		invalidateRules();
		if (editingRule?.domain === domain) setEditingRule(null);
	};

	const handleToggleRule = async (domain: string) => {
		const previous = queryClient.getQueryData<PagedResponse<DomainRule>>([
			'domain-rules',
			page,
			debouncedSearch,
		]);
		queryClient.setQueryData<PagedResponse<DomainRule>>(
			['domain-rules', page, debouncedSearch],
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
			await toggleRule.mutateAsync(domain);
		} catch (e) {
			queryClient.setQueryData(
				['domain-rules', page, debouncedSearch],
				previous,
			);
			toastError(e);
		}
	};

	const handleEditRule = async (domain: string, action: ListAction) => {
		await updateRule.mutateAsync({ domain, action }, { onError: toastError });
		invalidateRules();
		setEditingRule(null);
	};

	const handleAddSubscription = async (
		name: string,
		url: string,
		list_type: ListAction,
		sync_enabled: boolean,
	) => {
		await addSubscription.mutateAsync(
			{ name, url, list_type, sync_enabled },
			{ onError: toastError },
		);
		invalidateRules();
		invalidateSubs();
		setShowSubscriptionDialog(false);
	};

	const handleRemoveSubscription = async (id: string) => {
		await removeSubscription.mutateAsync(id, { onError: toastError });
		invalidateRules();
		invalidateSubs();
	};

	const handleToggleSubscription = async (id: string) => {
		const previous = queryClient.getQueryData<ListSubscription[]>([
			'list-subscriptions',
		]);
		queryClient.setQueryData<ListSubscription[]>(
			['list-subscriptions'],
			(old) =>
				old?.map((s) => (s.id === id ? { ...s, enabled: !s.enabled } : s)),
		);
		try {
			await toggleSubscription.mutateAsync(id);
		} catch (e) {
			queryClient.setQueryData(['list-subscriptions'], previous);
			toastError(e);
		}
	};

	const handleToggleSubscriptionSync = async (id: string) => {
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
			await toggleSubscriptionSync.mutateAsync(id);
		} catch (e) {
			queryClient.setQueryData(['list-subscriptions'], previous);
			toastError(e);
		}
	};

	const rules = rulesData?.items ?? [];

	return (
		<Box gap='8'>
			<HStack justify='space-between' mb='8'>
				<Heading size='lg'>Domain Rules</Heading>
			</HStack>
			<Tabs.Root defaultValue='rules' variant='line'>
				<Box
					display='flex'
					flexDir={{ base: 'column', sm: 'row' }}
					justifyContent='space-between'
					alignItems={{ base: 'flex-start', sm: 'center' }}
					gap={{ base: '3', sm: '0' }}
					mb='4'
					w='full'
				>
					<Tabs.List borderColor='border'>
						<Tabs.Trigger
							value='rules'
							color='fg.muted'
							_selected={{ color: 'accent.fg' }}
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
							_selected={{ color: 'accent.fg' }}
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
				<Tabs.Content value='rules' pt='0'>
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

				<Tabs.Content value='subscriptions' pt='0'>
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
