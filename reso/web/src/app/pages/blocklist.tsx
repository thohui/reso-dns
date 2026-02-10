import { Box, Button } from '@chakra-ui/react';
import { useState } from 'react';
import { getApiError } from '../..//lib/api/error';
import { BlocklistDialog } from '../../components/blocklist/BlocklistDialog';
import { BlocklistGrid } from '../../components/blocklist/BlocklistGrid';
import { toaster } from '../../components/Toaster';
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
		await createDomain.mutateAsync(domain, {
			onError: async (e) => {

				const error = await getApiError(e);

				if (error) {
					toaster.error({ title: 'Error', description: error.message, duration: 1000 });
				}

				else if (e instanceof Error) {
					toaster.error({ title: 'Error', description: e.message, duration: 1000 });
				}

				else {
					toaster.error({ title: 'Error', description: 'Something went wrong', duration: 1000 });
				}

			}
		});
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
