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
import { zodResolver } from '@hookform/resolvers/zod';
import { Key, X } from 'lucide-react';
import { useForm } from 'react-hook-form';
import z from 'zod';

interface CreateApiKeyDialogProps {
	onClose: () => void;
	onSubmit: (payload: { display_name: string; expires_at?: number; }) => Promise<void>;
}

const schema = z.object({
	display_name: z.string().min(1, 'Display name is required'),
	expires_days: z
		.number()
		.int()
		.min(1)
		.optional()
		.or(z.literal('').transform(() => undefined)),
});

export function CreateApiKeyDialog({ onClose, onSubmit }: CreateApiKeyDialogProps) {
	const {
		register,
		handleSubmit,
		setError,
		formState: { errors, isSubmitting },
	} = useForm({
		resolver: zodResolver(schema),
		defaultValues: { expires_days: '' as unknown as number },
	});

	const onSubmitHandler = handleSubmit(async (data) => {
		try {
			const expires_at = data.expires_days
				? Date.now() + data.expires_days * 24 * 60 * 60 * 1000
				: undefined;
			await onSubmit({ display_name: data.display_name, expires_at });
		} catch (e) {
			if (e instanceof Error) {
				setError('root', { message: e.message });
			}
		}
	});

	return (
		<Dialog.Root open onOpenChange={({ open }) => !open && onClose()}>
			<Dialog.Backdrop backdropFilter='blur(4px)' bg='blackAlpha.700' />
			<Dialog.Positioner>
				<Dialog.Content
					bg='bg.panel'
					borderColor='border'
					borderWidth='1px'
					maxW='420px'
					borderRadius='xl'
					boxShadow='dark-lg'
				>
					<form onSubmit={onSubmitHandler}>
						<Dialog.Header pt='6' px='6' pb='0'>
							<HStack justify='space-between' w='full'>
								<HStack gap='3'>
									<Icon as={Key} boxSize='5' color='accent.fg' />
									<Heading size='md'>New API Key</Heading>
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
							<Text color='fg.muted' fontSize='sm' mb='5'>
								API keys allow programmatic access to the Reso API. The key will
								only be shown once after creation.
							</Text>

							<Field.Root invalid={!!errors.display_name} mb='4'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Display Name
								</Field.Label>
								<Input
									placeholder='Reso CLI'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_hover={{ borderColor: 'accent.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									{...register('display_name')}
								/>
								{errors.display_name?.message && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{errors.display_name.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							<Field.Root invalid={!!errors.expires_days} mb='4'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Expires in (days)
								</Field.Label>
								<Input
									type='number'
									placeholder='Leave blank for no expiry'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_hover={{ borderColor: 'accent.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									{...register('expires_days')}
								/>
								{errors.expires_days?.message && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{errors.expires_days.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							{errors.root?.message && (
								<Text color='status.error' fontSize='xs' mb='4'>
									{errors.root.message}
								</Text>
							)}
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
								>
									Create Key
								</Button>
							</HStack>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
	);
}
