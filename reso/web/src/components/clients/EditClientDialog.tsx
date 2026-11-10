import {
	Button,
	Dialog,
	Field,
	Heading,
	HStack,
	Icon,
	IconButton,
	Input,
	Text,
} from '@chakra-ui/react';
import { Pencil, X } from 'lucide-react';
import { useState } from 'react';
import { FormError } from '@/components/FormError';
import type { Client } from '@/lib/api/clients';
import { getErrorMessage } from '@/lib/api/error';

const MAX_DISPLAY_NAME_LENGTH = 255;

interface EditClientDialogProps {
	client: Client;
	onClose: () => void;
	onSubmit: (ip: string, displayName: string | null) => Promise<void>;
}

export function EditClientDialog({
	client,
	onClose,
	onSubmit,
}: EditClientDialogProps) {
	const [displayName, setDisplayName] = useState(client.display_name ?? '');
	const [isSubmitting, setIsSubmitting] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const trimmed = displayName.trim();
	const unchanged = trimmed === (client.display_name ?? '');

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		setIsSubmitting(true);
		setError(null);
		try {
			await onSubmit(client.ip, trimmed || null);
		} catch (err) {
			setError(await getErrorMessage(err));
		} finally {
			setIsSubmitting(false);
		}
	};

	return (
		<Dialog.Root open onOpenChange={({ open }) => !open && onClose()}>
			<Dialog.Backdrop backdropFilter='blur(4px)' bg='blackAlpha.700' />
			<Dialog.Positioner>
				<Dialog.Content
					bg='bg.panel'
					borderColor='border'
					borderWidth='1px'
					maxW='480px'
					borderRadius='xl'
					boxShadow='dark-lg'
				>
					<form onSubmit={handleSubmit}>
						<Dialog.Header pt='6' px='6' pb='0'>
							<HStack justify='space-between' w='full'>
								<HStack gap='3'>
									<Icon as={Pencil} boxSize='5' color='accent.fg' />
									<Heading size='md'>Edit Client</Heading>
								</HStack>
								<IconButton
									aria-label='Close dialog'
									variant='ghost'
									size='sm'
									type='button'
									onClick={onClose}
									_hover={{ bg: 'bg.subtle' }}
								>
									<Icon as={X} boxSize='4' color='fg.muted' />
								</IconButton>
							</HStack>
						</Dialog.Header>

						<Dialog.Body px='6' pb='0' pt='4'>
							<Field.Root mb='5'>
								<Field.Label color='fg.muted' fontSize='sm'>
									IP Address
								</Field.Label>
								<Text
									fontFamily='data'
									fontSize='sm'
									fontWeight='500'
									color='fg'
									py='2'
									px='3'
									bg='bg.subtle'
									borderRadius='md'
									borderWidth='1px'
									borderColor='border'
									w='full'
								>
									{client.ip}
								</Text>
							</Field.Root>

							<Field.Root mb='5'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Display Name
								</Field.Label>
								<Input
									autoFocus
									value={displayName}
									maxLength={MAX_DISPLAY_NAME_LENGTH}
									onChange={(e) => {
										setDisplayName(e.target.value);
										setError(null);
									}}
									placeholder={client.hostname ?? 'Living room TV'}
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_hover={{ borderColor: 'accent.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
								/>
								<Field.HelperText color='fg.subtle' fontSize='xs'>
									Leave empty to use the hostname.
								</Field.HelperText>
							</Field.Root>

							<FormError message={error ?? undefined} />
						</Dialog.Body>

						<Dialog.Footer px='6' pb='6' pt='0' justifyContent='flex-end'>
							<HStack gap='3'>
								<Button
									variant='ghost'
									type='button'
									color='fg.muted'
									_hover={{ bg: 'bg.subtle' }}
									onClick={onClose}
									px='5'
								>
									Cancel
								</Button>
								<Button
									type='submit'
									bg='accent'
									color='fg'
									_hover={{ bg: 'accent.hover' }}
									px='5'
									loading={isSubmitting}
									disabled={unchanged}
								>
									Save
								</Button>
							</HStack>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
	);
}
