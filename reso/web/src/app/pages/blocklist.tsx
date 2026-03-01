import { Box, Button, Grid, Heading, HStack, Icon } from '@chakra-ui/react';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, Plus, ShieldCheck, ShieldOff } from 'lucide-react';
import { StatCard } from '../../components/dashboard/StatCard';
import { useState } from 'react';
import { BlocklistDialog } from '../../components/blocklist/BlocklistDialog';
import { BlocklistGrid } from '../../components/blocklist/BlocklistGrid';
import { toastError } from '../../components/Toaster';
import type { BlockedDomain } from '../../lib/api/blocklist';
import type { PagedResponse } from '../../lib/api/pagination';
import { useBlocklist } from '../../hooks/useBlocklist';
import { useCreateDomain } from '../../hooks/useCreateDomain';
import { useRemoveDomain } from '../../hooks/useRemoveDomain';
import { useToggleDomain } from '../../hooks/useToggleDomain';

export default function BlocklistPage() {
	const { data, refetch } = useBlocklist();
	const queryClient = useQueryClient();

	const [showDialog, setShowDialog] = useState(false);

	const handleClick = () => {
		setShowDialog(true);
	};

	const handleClose = () => {
		setShowDialog(false);
	};

	const createDomain = useCreateDomain();
	const removeDomain = useRemoveDomain();
	const toggleDomain = useToggleDomain();

	const handleSubmit = async (domain: string) => {
		await createDomain.mutateAsync(domain, {
			onError: (e) => toastError(e),
		});
		await refetch();
		handleClose();
	};

	const handleRemove = async (domain: string) => {
		await removeDomain.mutateAsync(domain, {
			onError: async (e) => {
				await Promise.all([toastError(e), refetch()]);
			},
		});
		await refetch();
	};

	const handleToggle = async (domain: string) => {
		const previous =
			queryClient.getQueryData<PagedResponse<BlockedDomain>>([
				'blocklist',
			]);

		queryClient.setQueryData<PagedResponse<BlockedDomain>>(
			['blocklist'],
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
			await toggleDomain.mutateAsync(domain);
		} catch (e) {
			queryClient.setQueryData(['blocklist'], previous);
			toastError(e);
		}
	};

	const items = data?.items ?? [];
	const totalCount = items.length;
	const enabledCount = items.filter((d) => d.enabled).length;
	const disabledCount = totalCount - enabledCount;

	return (
		<Box>
			<HStack justify='space-between' mb='8'>
				<Heading size='lg'>Blocklist</Heading>
				<Button
					bg='accent'
					color='fg'
					_hover={{ bg: 'accent.hover' }}
					h='9'
					fontSize='sm'
					onClick={handleClick}
				>
					<Icon as={Plus} boxSize='3.5' mr='2' />
					Add Domain
				</Button>
			</HStack>

			<Grid templateColumns='repeat(3, 1fr)' gap='4' mb='6'>
				<StatCard
					label='Total Domains'
					value={totalCount}
					icon={Ban}
					accentColor='status.error'
				/>
				<StatCard
					label='Active'
					value={enabledCount}
					icon={ShieldCheck}
					accentColor='status.success'
				/>
				<StatCard
					label='Disabled'
					value={disabledCount}
					icon={ShieldOff}
					accentColor='status.warn'
				/>
			</Grid>

			{showDialog && (
				<BlocklistDialog onClose={handleClose} onSubmit={handleSubmit} />
			)}
			<BlocklistGrid
				blocklist={items}
				onRemove={handleRemove}
				onToggle={handleToggle}
			/>
		</Box>
	);
}
