import { Box, Button, Heading, HStack, Icon } from '@chakra-ui/react';
import { Plus } from 'lucide-react';
import { useState } from 'react';
import { BlocklistDialog } from '../../components/blocklist/BlocklistDialog';
import { BlocklistGrid } from '../../components/blocklist/BlocklistGrid';
import { toastError } from '../../components/Toaster';
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
			onError: (e) => toastError(e),
		});
		await refetch();
		handleClose();
	};

	const handleRemove = async (domain: string) => {
		await removeDomain.mutateAsync(domain, {
			onError: async (e) => {
				await Promise.all([toastError(e), refetch()]);
			}
		});

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
						Add
					</Button>
				</HStack>
				{showDialog && (
					<BlocklistDialog onClose={handleClose} onSubmit={handleSubmit} />
				)}
				<Box display='flex' flexDirection='row-reverse'>
				</Box>
				<BlocklistGrid blocklist={data?.items ?? []} onRemove={handleRemove} />
			</Box>
		);
	};
}
