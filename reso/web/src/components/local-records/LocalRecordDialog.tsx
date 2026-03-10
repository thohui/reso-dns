import {
	Box,
	Button,
	Field,
	Heading,
	HStack,
	Icon,
	IconButton,
	Input,
	NativeSelect,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Plus, X } from 'lucide-react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import { LOCAL_RECORD_TYPES } from '../../lib/dns';

interface LocalRecordDialogProps {
	onClose: () => void;
	onSubmit: (record: { name: string; record_type: number; value: string; ttl?: number }) => Promise<void>;
}

const validTypeValues = LOCAL_RECORD_TYPES.map((t) => t.value);

const schema = z.object({
	name: z.string().min(1, 'Domain name is required'),
	record_type: z.coerce.number().refine((v) => validTypeValues.includes(v as typeof validTypeValues[number])),
	value: z.string().min(1, 'Value is required'),
	ttl: z.coerce.number().int().min(1).optional(),
});

const VALUE_PLACEHOLDERS: Record<number, string> = {
	1: 'e.g. 192.168.1.100',
	28: 'e.g. fd00::1',
	5: 'e.g. example.com',
};

export function LocalRecordDialog({ onClose, onSubmit }: LocalRecordDialogProps) {
	const {
		register,
		handleSubmit,
		setError,
		watch,
		formState: { errors, isSubmitting },
	} = useForm({
		resolver: zodResolver(schema),
		defaultValues: {
			name: '',
			record_type: 1,
			value: '',
			ttl: 300,
		},
	});

	const recordType = watch('record_type') as number;

	const onSubmitHandler = handleSubmit(async (data) => {
		try {
			await onSubmit(data);
		} catch (e) {
			if (e instanceof Error) {
				setError('root', { message: e.message });
			}
		}
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
					onClick={onClose}
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
							<Heading size='md'>Add Record</Heading>
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

					<Box px='6' pb='6' pt='4'>
						<Text color='fg.muted' fontSize='sm' mb='5'>
							Add a local DNS record. Queries matching this record will be answered directly by Reso.
						</Text>

						<HStack gap='3' mb='4' align='flex-start'>
							<Field.Root invalid={!!errors.name} flex='1'>
								<Field.Label color='fg.muted' fontSize='sm'>Domain</Field.Label>
								<Input
									placeholder='e.g. myapp.home'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									autoFocus
									{...register('name')}
								/>
								{errors.name?.message && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{errors.name.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							<Field.Root w='120px'>
								<Field.Label color='fg.muted' fontSize='sm'>Type</Field.Label>
								<NativeSelect.Root>
									<NativeSelect.Field
										bg='bg.input'
										borderColor='border.input'
										{...register('record_type', { valueAsNumber: true })}
									>
										{LOCAL_RECORD_TYPES.map((t) => (
											<option key={t.value} value={t.value}>{t.label}</option>
										))}
									</NativeSelect.Field>
									<NativeSelect.Indicator />
								</NativeSelect.Root>
							</Field.Root>
						</HStack>

						<HStack gap='3' mb='6' align='flex-start'>
							<Field.Root invalid={!!errors.value} flex='1'>
								<Field.Label color='fg.muted' fontSize='sm'>Value</Field.Label>
								<Input
									placeholder={VALUE_PLACEHOLDERS[recordType] ?? ''}
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									{...register('value')}
								/>
								{errors.value?.message && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{errors.value.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							<Field.Root w='100px'>
								<Field.Label color='fg.muted' fontSize='sm'>TTL</Field.Label>
								<Input
									type='number'
									bg='bg.input'
									borderColor='border.input'
									{...register('ttl', { valueAsNumber: true })}
								/>
							</Field.Root>
						</HStack>

						{errors.root?.message && (
							<Text color='status.error' fontSize='xs' mb='4'>
								{errors.root.message}
							</Text>
						)}

						<HStack justify='flex-end' gap='3'>
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
								Add Record
							</Button>
						</HStack>
					</Box>
				</Box>
			</Box>
		</form>
	);
}
