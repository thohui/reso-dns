import { Button, HStack, Icon, IconButton, Table } from '@chakra-ui/react';
import { Trash2 } from 'lucide-react';
import { useState } from 'react';

interface Props {
	onConfirm: () => void;
}

export function ConfirmDeleteButton({ onConfirm }: Props) {
	const [confirming, setConfirming] = useState(false);

	return (
		<Table.Cell py='3.5' px='4'>
			{confirming ? (
				<HStack gap='1'>
					<Button
						size='md'
						variant='ghost'
						color='status.error'
						bg='status.errorMuted'
						_hover={{ opacity: 0.8 }}
						onClick={() => {
							onConfirm();
							setConfirming(false);
						}}
					>
						Confirm
					</Button>
					<Button
						size='md'
						variant='ghost'
						color='fg.muted'
						_hover={{ bg: 'bg.subtle' }}
						onClick={() => setConfirming(false)}
					>
						Cancel
					</Button>
				</HStack>
			) : (
				<IconButton
					size='md'
					cursor='pointer'
					variant='ghost'
					display='inline-flex'
					borderRadius='md'
					color='fg.subtle'
					_hover={{ color: 'status.error', bg: 'status.errorMuted' }}
					transition='all 0.15s'
					onClick={() => setConfirming(true)}
				>
					<Icon as={Trash2} boxSize='3.5' />
				</IconButton>
			)}
		</Table.Cell>
	);
}
