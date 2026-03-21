import {
	Button,
	Dialog,
	Field,
	HStack,
	Heading,
	Icon,
	IconButton,
	Input,
	Switch,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Plus, X } from 'lucide-react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import type { ListAction } from '../../lib/api/domain-rules';

interface SubscriptionDialogProps {
	onClose: () => void;
	onSubmit: (
		name: string,
		url: string,
		list_type: ListAction,
		sync_enabled: boolean,
	) => Promise<void>;
}

const schema = z.object({
	name: z.string().min(1, 'Name is required'),
	url: z.url('Must be a valid URL'),
	sync_enabled: z.boolean().optional().default(true),
});

export function SubscriptionDialog({
	onClose,
	onSubmit,
}: SubscriptionDialogProps) {

	const form = useForm({
		resolver: zodResolver(schema),
		defaultValues: { sync_enabled: true }
	});

	const onSubmitHandler = form.handleSubmit(
		async ({ name, url, sync_enabled }) => {
			try {
				await onSubmit(name, url, 'block', sync_enabled);
			} catch (e) {
				if (e instanceof Error) {
					form.setError('root', { message: e.message });
				}
			}
		},
	);

	const syncEnabled = !!form.watch('sync_enabled');

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
					<form onSubmit={onSubmitHandler}>
						<Dialog.Header pt='6' px='6' pb='0'>
							<HStack justify='space-between' w='full'>
								<HStack gap='3'>
									<Icon as={Plus} boxSize='5' color='accent.fg' />
									<Heading size='md'>Add Subscription</Heading>
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
							<Field.Root invalid={!!form.formState.errors.name} mb='4'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Name
								</Field.Label>
								<Input
									placeholder='e.g. OISD Full'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									autoFocus
									{...form.register('name')}
								/>
								{form.formState.errors.name?.message && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{form.formState.errors.name.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							<Field.Root
								invalid={
									!!form.formState.errors.url || !!form.formState.errors.root
								}
								mb='6'
							>
								<Field.Label color='fg.muted' fontSize='sm'>
									URL
								</Field.Label>
								<Input
									placeholder='https://example.com/list.txt'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									{...form.register('url')}
								/>
								{(form.formState.errors.url?.message ||
									form.formState.errors.root?.message) && (
										<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
											{form.formState.errors.url?.message ??
												form.formState.errors.root?.message}
										</Field.ErrorText>
									)}
							</Field.Root>

							<Field.Root mb='6'>
								<HStack justify='space-between'>
									<Field.Label color='fg.muted' fontSize='sm' mb='0'>
										Sync periodically
									</Field.Label>
									<Switch.Root
										checked={syncEnabled}
										onCheckedChange={({ checked }) =>
											form.setValue('sync_enabled', checked)
										}
									>
										<Switch.HiddenInput />
										<Switch.Control
											bg={syncEnabled ? 'accent' : 'bg.elevated'}
											borderWidth='1px'
											borderColor={syncEnabled ? 'accent' : 'border.input'}
											_hover={{ borderColor: 'accent' }}
											transition='all 0.2s ease'
										>
											<Switch.Thumb bg='fg' />
										</Switch.Control>
									</Switch.Root>
								</HStack>
							</Field.Root>
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
									loading={form.formState.isSubmitting}
								>
									Add Subscription
								</Button>
							</HStack>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
	);
}
