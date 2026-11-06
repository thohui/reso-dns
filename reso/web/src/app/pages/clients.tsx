import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ClientsGrid } from '@/components/clients/ClientsGrid';
import { EditClientDialog } from '@/components/clients/EditClientDialog';
import { CLIENTS_PAGE_SIZE, useClients } from '@/hooks/clients/useClients';
import { useUpdateClient } from '@/hooks/clients/useUpdateClient';
import { useDebounce } from '@/hooks/useDebounce';
import type { Client } from '@/lib/api/clients';

export default function ClientsPage() {
	const [page, setPage] = useState(0);
	const [search, setSearch] = useState('');
	const [editing, setEditing] = useState<Client | null>(null);

	const debouncedSearch = useDebounce(search, 300);

	const { data, isFetching } = useClients(page, debouncedSearch);
	const queryClient = useQueryClient();
	const updateMutation = useUpdateClient();

	const total = data?.total;
	const totalPages =
		total != null ? Math.max(1, Math.ceil(total / CLIENTS_PAGE_SIZE)) : null;

	const handleSearchChange = (value: string) => {
		setSearch(value);
		setPage(0);
	};

	const handleUpdate = async (ip: string, displayName: string | null) => {
		await updateMutation.mutateAsync({ ip, display_name: displayName });
		queryClient.invalidateQueries({ queryKey: ['clients'] });
		setEditing((current) => (current?.ip === ip ? null : current));
	};

	return (
		<>
			{editing && (
				<EditClientDialog
					client={editing}
					onClose={() => setEditing(null)}
					onSubmit={handleUpdate}
				/>
			)}
			<ClientsGrid
				clients={data?.items ?? []}
				onEdit={setEditing}
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
