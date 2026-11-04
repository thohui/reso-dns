import { Button, HStack, Icon, IconButton, Table } from '@chakra-ui/react';
import { Trash2 } from 'lucide-react';
import { useState } from 'react';

interface Props {
	onConfirm: () => void;
}

export function ConfirmDeleteButton({ onConfirm }: Props) {
	const [confirming, setConfirming] = useState(false);

	return (
		<Table.Cell py='3' px='4'>
			{confirming ? (
				<HStack gap='1'>
					<Button
						size='xs'
						h='6'
						variant='ghost'
						color='status.error'
						bg='status.errorMuted'
						_hover={{ opacity: 0.8 }}
						onClick={(e) => {
							e.stopPropagation();
							onConfirm();
							setConfirming(false);
						}}
					>
						Confirm
					</Button>
					<Button
						size='xs'
						h='6'
						variant='ghost'
						color='fg.muted'
						_hover={{ bg: 'bg.subtle' }}
						onClick={(e) => {
							e.stopPropagation();
							setConfirming(false);
						}}
					>
						Cancel
					</Button>
				</HStack>
			) : (
				<IconButton
					size='2xs'
					aria-label='Delete'
					cursor='pointer'
					variant='ghost'
					display='inline-flex'
					borderRadius='md'
					color='fg.subtle'
					_hover={{ color: 'status.error', bg: 'status.errorMuted' }}
					transition='all 0.15s'
					onClick={(e) => {
						e.stopPropagation();
						setConfirming(true);
					}}
				>
					<Icon as={Trash2} boxSize='3.5' />
				</IconButton>
			)}
		</Table.Cell>
	);
}
