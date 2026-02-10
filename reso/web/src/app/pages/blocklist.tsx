'use client';

import { Box, Button } from '@chakra-ui/react';
import { useState } from 'react';
import { BlocklistDialog } from '../../components/blocklist/BlocklistDialog';
import { BlocklistGrid } from '../../components/blocklist/BlocklistGrid';
import { useBlocklist } from '../../hooks/useBlocklist';
import { useCreateDomain } from '../../hooks/useCreateDomain';
import { useRemoveDomain } from '../../hooks/useRemoveDomain';

export default function BlocklistPage() {
	const { data, refetch } = useBlocklist();

	const [showDialog, setShowDialog] = useState(false);

	const handleClick = () => {
		setShowDialog(true);
	};

	const handleClose = () => {
		setShowDialog(false);
	};

	const createDomain = useCreateDomain();
	const removeDomain = useRemoveDomain();

	const handleSubmit = async (domain: string) => {
		await createDomain.mutateAsync(domain);
		await refetch();
		handleClose();
	};

	const handleRemove = async (domain: string) => {
		await removeDomain.mutateAsync(domain);
		await refetch();
	};

	return (
		<Box>
			{showDialog && (
				<BlocklistDialog onClose={handleClose} onSubmit={handleSubmit} />
			)}
			<Box display='flex' flexDirection='row-reverse'>
				<Button bgColor='green.600' mb={2} onClick={handleClick}>
					Add
				</Button>
			</Box>
			<BlocklistGrid blocklist={data?.items ?? []} onRemove={handleRemove} />
		</Box>
	);
}
