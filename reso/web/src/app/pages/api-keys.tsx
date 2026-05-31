import { ApiKeyCreatedDialog } from '@/components/api-keys/ApiKeyCreatedDialog';
import { ApiKeysGrid } from '@/components/api-keys/ApiKeysGrid';
import { CreateApiKeyDialog } from '@/components/api-keys/CreateApiKeyDialog';
import { toastError } from '@/components/Toaster';
import { apiKeysQueryKey, useApiKeys } from '@/hooks/api-keys/useApiKeys';
import { useCreateApiKey } from '@/hooks/api-keys/useCreateApiKey';
import { useDeleteApiKey } from '@/hooks/api-keys/useDeleteApiKey';
import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

export default function ApiKeysPage() {
	const { data, refetch } = useApiKeys();
	const queryClient = useQueryClient();

	const createMutation = useCreateApiKey();
	const deleteMutation = useDeleteApiKey();

	const [showCreate, setShowCreate] = useState(false);
	const [createdKeyId, setCreatedKeyId] = useState<string | null>(null);

	const handleCreate = async (payload: { display_name: string, expires_at?: number; }) => {
		const key = await createMutation.mutateAsync(payload);
		await refetch();
		setShowCreate(false);
		setCreatedKeyId(key.token);
	};

	const handleDelete = async (id: string) => {
		queryClient.setQueryData(apiKeysQueryKey, (old: typeof data) => {
			if (!old) return old;
			return { ...old, items: old.items.filter((k) => k.id !== id) };
		});

		try {
			await deleteMutation.mutateAsync(id);
		} catch (e) {
			await refetch();
			toastError(e);
		}
	};

	return (
		<>
			{showCreate && (
				<CreateApiKeyDialog
					onClose={() => setShowCreate(false)}
					onSubmit={handleCreate}
				/>
			)}
			{createdKeyId && (
				<ApiKeyCreatedDialog
					keyId={createdKeyId}
					onClose={() => setCreatedKeyId(null)}
				/>
			)}
			<ApiKeysGrid
				keys={data?.items ?? []}
				onDelete={handleDelete}
				onAdd={() => setShowCreate(true)}
			/>
		</>
	);
}
