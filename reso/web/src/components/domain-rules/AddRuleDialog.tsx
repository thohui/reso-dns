import {
	Button,
	Dialog,
	Field,
	HStack,
	Heading,
	Icon,
	IconButton,
	Input,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Ban, Plus, ShieldCheck, X } from 'lucide-react';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import type { ListAction } from '../../lib/api/domain-rules';

interface AddRuleDialogProps {
	onClose: () => void;
	onSubmit: (domain: string, action: ListAction) => Promise<void>;
}

const schema = z.object({
	domain: z.string().min(1),
});

export function AddRuleDialog({ onClose, onSubmit }: AddRuleDialogProps) {
	const [action, setAction] = useState<ListAction>('block');

	const {
		register,
		handleSubmit,
		setError,
		formState: { errors, isSubmitting },
	} = useForm({ resolver: zodResolver(schema) });

	const onSubmitHandler = handleSubmit(async ({ domain }) => {
		try {
			await onSubmit(domain, action);
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
					maxW='480px'
					borderRadius='xl'
					boxShadow='dark-lg'
				>
					<form onSubmit={onSubmitHandler}>
						<Dialog.Header pt='6' px='6' pb='0'>
							<HStack justify='space-between' w='full'>
								<HStack gap='3'>
									<Icon as={Plus} boxSize='5' color='accent.fg' />
									<Heading size='md'>Add Domain Rule</Heading>
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
									Action
								</Field.Label>
								<HStack gap='2'>
									<Button
										type='button'
										size='sm'
										flex='1'
										variant='outline'
										bg={action === 'block' ? 'status.errorMuted' : 'bg.panel'}
										borderColor={
											action === 'block' ? 'status.error' : 'border.input'
										}
										color={action === 'block' ? 'status.error' : 'fg.muted'}
										_hover={{
											borderColor: 'status.error',
											color: 'status.error',
										}}
										onClick={() => setAction('block')}
									>
										<Icon as={Ban} boxSize='3.5' mr='2' />
										Block
									</Button>
									<Button
										type='button'
										size='sm'
										flex='1'
										variant='outline'
										bg={action === 'allow' ? 'status.successMuted' : 'bg.panel'}
										borderColor={
											action === 'allow' ? 'status.success' : 'border.input'
										}
										color={action === 'allow' ? 'status.success' : 'fg.muted'}
										_hover={{
											borderColor: 'status.success',
											color: 'status.success',
										}}
										onClick={() => setAction('allow')}
									>
										<Icon as={ShieldCheck} boxSize='3.5' mr='2' />
										Allow
									</Button>
								</HStack>
							</Field.Root>

							<Field.Root invalid={!!errors.root || !!errors.domain} mb='6'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Domain
								</Field.Label>
								<Input
									placeholder='e.g. ads.example.com or *.example.com'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									autoFocus
									{...register('domain')}
								/>
								{(errors.domain?.message || errors.root?.message) && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{errors.domain?.message ?? errors.root?.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							<Text color='fg.muted' fontSize='xs' mb='5'>
								{action === 'block'
									? 'All DNS queries to this domain will be denied.'
									: 'This domain will always resolve, bypassing any block rules.'}
							</Text>
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
									bg={action === 'block' ? 'status.error' : 'status.success'}
									color='white'
									_hover={{ opacity: 0.85 }}
									px='5'
									loading={isSubmitting}
								>
									{action === 'block' ? 'Block Domain' : 'Allow Domain'}
								</Button>
							</HStack>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
	);
}
