import {
	Button,
	chakra,
	Dialog,
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
import { type ReactNode, useState } from 'react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import { FormError } from '@/components/FormError';
import { getErrorMessage } from '@/lib/api/error';
import type { LocalRecordMatchType } from '@/lib/api/local-records';
import { LOCAL_RECORD_TYPES } from '@/lib/dns';

interface LocalRecordDialogProps {
	onClose: () => void;
	onSubmit: (record: {
		name: string;
		match_type: LocalRecordMatchType;
		record_type: number;
		value: string;
		ttl?: number;
	}) => Promise<void>;
}

const validTypeValues = LOCAL_RECORD_TYPES.map((t) => t.value);

const schema = z.object({
	name: z.string().min(1, 'Domain name is required'),
	record_type: z.coerce
		.number()
		.refine((v) =>
			validTypeValues.includes(v as (typeof validTypeValues)[number]),
		),
	value: z.string().min(1, 'Value is required'),
	ttl: z.coerce.number().int().min(1).optional(),
});

const matchTypeOptions: { value: LocalRecordMatchType; label: string }[] = [
	{ value: 'exact', label: 'Exact' },
	{ value: 'wildcard', label: 'Wildcard' },
];

function D({ children }: { children: ReactNode }) {
	return (
		<chakra.span fontFamily='monospace' color='fg' fontWeight='500'>
			{children}
		</chakra.span>
	);
}

function getMatchDescription(
	name: string,
	matchType: LocalRecordMatchType,
): ReactNode {
	const d = name.trim().replace(/^\*\./, '') || 'myapp.home';
	if (matchType === 'wildcard') {
		return (
			<>
				Answers subdomains like <D>app.{d}</D> and <D>nas.{d}</D>, but not{' '}
				<D>{d}</D> itself.
			</>
		);
	}
	return (
		<>
			Answers <D>{d}</D> only.
		</>
	);
}

const VALUE_PLACEHOLDERS: Record<number, string> = {
	1: 'e.g. 192.168.1.100',
	28: 'e.g. fd00::1',
	5: 'e.g. example.com',
};

export function LocalRecordDialog({
	onClose,
	onSubmit,
}: LocalRecordDialogProps) {
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

	const [matchType, setMatchType] = useState<LocalRecordMatchType>('exact');
	const recordType = watch('record_type') as number;
	const nameValue = watch('name');

	const onSubmitHandler = handleSubmit(async (data) => {
		try {
			const name = data.name.trim().toLowerCase();
			const bare = name.replace(/^\*\./, '');
			await onSubmit({
				...data,
				name: bare,
				match_type: bare !== name ? 'wildcard' : matchType,
			});
		} catch (e) {
			setError('root', { message: await getErrorMessage(e) });
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
						</Dialog.Header>

						<Dialog.Body px='6' pb='0' pt='4'>
							<Text color='fg.muted' fontSize='sm' mb='5'>
								Add a local DNS record. Queries matching this record will be
								answered directly by Reso.
							</Text>

							<HStack gap='3' mb='4' align='flex-start'>
								<Field.Root invalid={!!errors.name} flex='1'>
									<Field.Label color='fg.muted' fontSize='sm'>
										Domain
									</Field.Label>
									<Input
										placeholder='e.g. myapp.home'
										bg='bg.input'
										borderColor='border.input'
										_placeholder={{ color: 'fg.subtle' }}
										_hover={{ borderColor: 'accent.subtle' }}
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
									<Field.Label color='fg.muted' fontSize='sm'>
										Type
									</Field.Label>
									<NativeSelect.Root>
										<NativeSelect.Field
											bg='bg.input'
											borderColor='border.input'
											_hover={{ borderColor: 'accent.subtle' }}
											{...register('record_type', { valueAsNumber: true })}
										>
											{LOCAL_RECORD_TYPES.map((t) => (
												<option key={t.value} value={t.value}>
													{t.label}
												</option>
											))}
										</NativeSelect.Field>
										<NativeSelect.Indicator />
									</NativeSelect.Root>
								</Field.Root>
							</HStack>

							<Field.Root mb='4'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Match
								</Field.Label>
								<HStack gap='2'>
									{matchTypeOptions.map(({ value, label }) => (
										<Button
											key={value}
											type='button'
											onClick={() => setMatchType(value)}
											aria-pressed={matchType === value}
											size='xs'
											borderRadius='md'
											borderWidth='1px'
											bg={matchType === value ? 'accent.muted' : 'bg.subtle'}
											color={matchType === value ? 'accent.fg' : 'fg.muted'}
											borderColor={
												matchType === value ? 'accent.subtle' : 'border.input'
											}
											_hover={{
												bg: 'accent.muted',
												color: 'accent.fg',
												borderColor: 'accent.subtle',
											}}
										>
											{label}
										</Button>
									))}
								</HStack>
								<Text color='fg.subtle' fontSize='xs' mt='2'>
									{getMatchDescription(nameValue ?? '', matchType)}
								</Text>
							</Field.Root>

							<HStack gap='3' mb='6' align='flex-start'>
								<Field.Root invalid={!!errors.value} flex='1'>
									<Field.Label color='fg.muted' fontSize='sm'>
										Value
									</Field.Label>
									<Input
										placeholder={VALUE_PLACEHOLDERS[recordType] ?? ''}
										bg='bg.input'
										borderColor='border.input'
										_placeholder={{ color: 'fg.subtle' }}
										_hover={{ borderColor: 'accent.subtle' }}
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
									<Field.Label color='fg.muted' fontSize='sm'>
										TTL
									</Field.Label>
									<Input
										type='number'
										bg='bg.input'
										borderColor='border.input'
										_hover={{ borderColor: 'accent.subtle' }}
										_focus={{ borderColor: 'accent.subtle' }}
										{...register('ttl', { valueAsNumber: true })}
									/>
								</Field.Root>
							</HStack>

							<FormError message={errors.root?.message} />
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
									Add Record
								</Button>
							</HStack>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
	);
}
