import {
	Box,
	Button,
	Field,
	Heading,
	HStack,
	Icon,
	IconButton,
	Input,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Plus, X } from 'lucide-react';
import { useForm } from 'react-hook-form';
import z from 'zod';

interface BlocklistDialogProps {
	onClose: () => void;
	onSubmit: (domain: string) => Promise<void>;
}

const schema = z.object({
	domain: z.string().nonempty(),
});

export function BlocklistDialog({ onClose, onSubmit }: BlocklistDialogProps) {
	const {
		register,
		handleSubmit,
		formState: { errors, isSubmitting },
	} = useForm({ resolver: zodResolver(schema) });

	const handleClose = () => {
		onClose();
	};

	const onSubmitHandler = handleSubmit(async ({ domain }) => {
		await onSubmit(domain);
	});

	return (
		<form onSubmit={onSubmitHandler}>
			<Box
				position='fixed'
				inset='0'
				zIndex='1000'
				display='flex'
				alignItems='center'
				justifyContent='center'
			>
				<Box
					position='absolute'
					inset='0'
					bg='blackAlpha.700'
					backdropFilter='blur(4px)'
					onClick={handleClose}
				/>

				<Box
					position='relative'
					bg='bg.panel'
					borderColor='border'
					borderWidth='1px'
					maxW='480px'
					w='full'
					mx='4'
					borderRadius='xl'
					boxShadow='dark-lg'
				>
					<HStack justify='space-between' pt='6' px='6' pb='0'>
						<HStack gap='3'>
							<Icon as={Plus} boxSize='5' color='accent.fg' />
							<Heading size='md'>Add Domain</Heading>
						</HStack>
						<IconButton
							aria-label='Close dialog'
							variant='ghost'
							size='sm'
							type='button'
							onClick={handleClose}
							_hover={{
								bg: 'bg.subtle'
							}}
						>
							<Icon as={X} boxSize='4' color='fg.muted' />
						</IconButton>
					</HStack>

					<Box px='6' pb='6' pt='4'>
						<Text color='fg.muted' fontSize='sm' mb='5'>
							Enter the domain you want to block. All DNS queries to this domain
							will be denied.
						</Text>

						<Field.Root invalid={!!errors.root || !!errors.domain} mb='6'>
							<Field.Label color='fg.muted' fontSize='sm'>
								Domain
							</Field.Label>
							<Input
								placeholder='e.g. ads.example.com'
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

						<HStack justify='flex-end' gap='3'>
							<Button
								variant='ghost'
								type='button'
								color='fg.muted'
								_hover={{ bg: 'bg.subtle' }}
								onClick={handleClose}
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
								Block Domain
							</Button>
						</HStack>
					</Box>
				</Box>
			</Box>
		</form >
	);
}
