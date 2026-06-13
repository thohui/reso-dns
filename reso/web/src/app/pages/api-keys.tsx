import { ApiKeyCreatedDialog } from '@/components/api-keys/ApiKeyCreatedDialog';
import { ApiKeysGrid } from '@/components/api-keys/ApiKeysGrid';
import { CreateApiKeyDialog } from '@/components/api-keys/CreateApiKeyDialog';
import { toastError } from '@/components/Toaster';
import { useDebounce } from '@/hooks/useDebounce';
import {
  API_KEYS_PAGE_SIZE,
  apiKeysQueryKey,
  useApiKeys,
} from '@/hooks/api-keys/useApiKeys';
import { useCreateApiKey } from '@/hooks/api-keys/useCreateApiKey';
import { useDeleteApiKey } from '@/hooks/api-keys/useDeleteApiKey';
import { useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';

export default function ApiKeysPage() {
  const [page, setPage] = useState(0);
  const [search, setSearch] = useState('');
  const cachedTotal = useRef<number | null>(null);

  const debouncedSearch = useDebounce(search, 300);

  const { data, refetch, isFetching } = useApiKeys(page, debouncedSearch);
  const queryClient = useQueryClient();

  const createMutation = useCreateApiKey();
  const deleteMutation = useDeleteApiKey();

  const [showCreate, setShowCreate] = useState(false);
  const [createdToken, setCreatedToken] = useState<string | null>(null);

  if (data?.total != null) {
    cachedTotal.current = data.total;
  }

  const total = data?.total ?? cachedTotal.current;
  const totalPages =
    total != null ? Math.max(1, Math.ceil(total / API_KEYS_PAGE_SIZE)) : null;

  const handleSearchChange = (value: string) => {
    setSearch(value);
    setPage(0);
  };

  const handleCreate = async (payload: {
    display_name: string;
    expires_at?: number;
  }) => {
    try {
      const key = await createMutation.mutateAsync(payload);
      setShowCreate(false);
      setCreatedToken(key.token);
      refetch();
    } catch (e) {
      toastError(e);
    }
  };

  const handleDelete = async (id: string) => {
    const previous = queryClient.getQueryData(
      apiKeysQueryKey(page, debouncedSearch),
    );
    queryClient.setQueryData(
      apiKeysQueryKey(page, debouncedSearch),
      (old: typeof data) => {
        if (!old) return old;
        return {
          ...old,
          total: old.total != null ? Math.max(0, old.total - 1) : old.total,
          items: old.items.filter((k) => k.id !== id),
        };
      },
    );

    try {
      await deleteMutation.mutateAsync(id);
      await refetch();
    } catch (e) {
      queryClient.setQueryData(
        apiKeysQueryKey(page, debouncedSearch),
        previous,
      );
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
      {createdToken && (
        <ApiKeyCreatedDialog
          token={createdToken}
          onClose={() => setCreatedToken(null)}
        />
      )}
      <ApiKeysGrid
        keys={data?.items ?? []}
        onDelete={handleDelete}
        onAdd={() => setShowCreate(true)}
        page={page}
        totalPages={totalPages}
        total={total}
        hasMore={data?.has_more}
        isLoading={isFetching}
        onPageChange={setPage}
        search={search}
        onSearchChange={handleSearchChange}
      />
    </>
  );
}
