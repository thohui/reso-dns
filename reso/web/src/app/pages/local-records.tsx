import { Box } from '@chakra-ui/react';
import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { LocalRecordDialog } from '@/components/local-records/LocalRecordDialog';
import { LocalRecordsGrid } from '@/components/local-records/LocalRecordsGrid';
import { toastError } from '@/components/Toaster';
import { useCreateLocalRecord } from '@/hooks/local-records/useCreateLocalRecord';
import {
	localRecordsQueryKey,
	useLocalRecords,
} from '@/hooks/local-records/useLocalRecords';
import { useRemoveLocalRecord } from '@/hooks/local-records/useRemoveLocalRecord';
import { useToggleLocalRecord } from '@/hooks/local-records/useToggleLocalRecord';
import type { LocalRecord } from '@/lib/api/local-records';
import type { PagedResponse } from '@/lib/api/pagination';

export default function LocalRecordsPage() {
	const { data, isFetching } = useLocalRecords();
	const queryClient = useQueryClient();

	const createMutation = useCreateLocalRecord();
	const removeMutation = useRemoveLocalRecord();
	const toggleMutation = useToggleLocalRecord();

	const [showDialog, setShowDialog] = useState(false);

	const invalidate = () =>
		queryClient.invalidateQueries({ queryKey: localRecordsQueryKey });

	const handleSubmit = async (record: {
		name: string;
		record_type: number;
		value: string;
		ttl?: number;
	}) => {
		await createMutation.mutateAsync(record);
		invalidate();
		setShowDialog(false);
	};

	const handleRemove = async (id: number) => {
		try {
			await removeMutation.mutateAsync(id);
			invalidate();
		} catch (e) {
			toastError(e);
		}
	};

	const handleToggle = async (id: number) => {
		const previous =
			queryClient.getQueryData<PagedResponse<LocalRecord>>(
				localRecordsQueryKey,
			);

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

	return (
		<Box display='flex' flexDir='column' flex='1' minH='0'>
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
				onAdd={() => setShowDialog(true)}
				isLoading={isFetching}
			/>
		</Box>
	);
}
