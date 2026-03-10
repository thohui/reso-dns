import { Box, Button, Grid, Heading, HStack, Icon } from '@chakra-ui/react';
import { useQueryClient } from '@tanstack/react-query';
import { Globe, Plus, ToggleLeft, ToggleRight } from 'lucide-react';
import { useState } from 'react';
import { StatCard } from '../../components/dashboard/StatCard';
import { LocalRecordDialog } from '../../components/local-records/LocalRecordDialog';
import { LocalRecordsGrid } from '../../components/local-records/LocalRecordsGrid';
import { toastError } from '../../components/Toaster';
import type { LocalRecord } from '../../lib/api/local-records';
import type { PagedResponse } from '../../lib/api/pagination';
import { localRecordsQueryKey, useLocalRecords } from '../../hooks/useLocalRecords';
import { useCreateLocalRecord } from '../../hooks/useCreateLocalRecord';
import { useRemoveLocalRecord } from '../../hooks/useRemoveLocalRecord';
import { useToggleLocalRecord } from '../../hooks/useToggleLocalRecord';

export default function LocalRecordsPage() {
	const { data, refetch } = useLocalRecords();
	const queryClient = useQueryClient();

	const createMutation = useCreateLocalRecord();
	const removeMutation = useRemoveLocalRecord();
	const toggleMutation = useToggleLocalRecord();

	const [showDialog, setShowDialog] = useState(false);

	const handleSubmit = async (record: { name: string; record_type: number; value: string; ttl?: number }) => {
		try {
			await createMutation.mutateAsync(record);
			await refetch();
			setShowDialog(false);
		} catch (e) {
			toastError(e);
			throw e;
		}
	};

	const handleRemove = async (id: number) => {
		try {
			await removeMutation.mutateAsync(id);
			await refetch();
		} catch (e) {
			toastError(e);
		}
	};

	const handleToggle = async (id: number) => {
		const previous = queryClient.getQueryData<PagedResponse<LocalRecord>>(localRecordsQueryKey);

		queryClient.setQueryData<PagedResponse<LocalRecord>>(
			localRecordsQueryKey,
			(old) => {
				if (!old) return old;
				return {
					...old,
					items: old.items.map((r) =>
						r.id === id ? { ...r, enabled: !r.enabled } : r,
					),
				};
			},
		);

		try {
			await toggleMutation.mutateAsync(id);
		} catch (e) {
			queryClient.setQueryData(localRecordsQueryKey, previous);
			toastError(e);
		}
	};

	const items = data?.items ?? [];
	const totalCount = items.length;
	const enabledCount = items.filter((r) => r.enabled).length;
	const disabledCount = totalCount - enabledCount;

	return (
		<Box>
			<HStack justify='space-between' mb='8'>
				<Heading size='lg'>Local Records</Heading>
				<Button
					bg='accent'
					color='fg'
					_hover={{ bg: 'accent.hover' }}
					h='9'
					fontSize='sm'
					onClick={() => setShowDialog(true)}
				>
					<Icon as={Plus} boxSize='3.5' mr='2' />
					Add Record
				</Button>
			</HStack>

			<Grid templateColumns='repeat(3, 1fr)' gap='4' mb='6'>
				<StatCard
					label='Total Records'
					value={totalCount}
					icon={Globe}
					accentColor='status.info'
				/>
				<StatCard
					label='Active'
					value={enabledCount}
					icon={ToggleRight}
					accentColor='status.success'
				/>
				<StatCard
					label='Disabled'
					value={disabledCount}
					icon={ToggleLeft}
					accentColor='status.warn'
				/>
			</Grid>

			{showDialog && (
				<LocalRecordDialog
					onClose={() => setShowDialog(false)}
					onSubmit={handleSubmit}
				/>
			)}
			<LocalRecordsGrid
				records={items}
				onRemove={handleRemove}
				onToggle={handleToggle}
			/>
		</Box>
	);
}
