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
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Plus, X } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { useForm, useWatch } from 'react-hook-form';
import z from 'zod';
import { ActionBadge } from '@/components/ActionBadge';
import { FormError } from '@/components/FormError';
import type { ListAction, MatchType } from '@/lib/api/domain-rules';
import { getErrorMessage } from '@/lib/api/error';

interface AddRuleDialogProps {
	onClose: () => void;
	onSubmit: (
		domain: string,
		matchType: MatchType,
		action: ListAction,
	) => Promise<void>;
}

const schema = z.object({
	domain: z.string().trim().min(1),
});

const matchTypeOptions: { value: MatchType; label: string }[] = [
	{ value: 'domain', label: 'Domain' },
	{ value: 'wildcard', label: 'Wildcard' },
	{ value: 'exact', label: 'Exact' },
];

function D({ children }: { children: ReactNode }) {
	return (
		<chakra.span fontFamily='monospace' color='fg' fontWeight='500'>
			{children}
		</chakra.span>
	);
}

function getMatchDescription(
	domain: string,
	matchType: MatchType,
	action: ListAction,
): ReactNode {
	const d = domain.trim().replace(/^\*\./, '') || 'example.com';
	const verb = action === 'block' ? 'Blocks' : 'Allows';
	const verb2 = action === 'block' ? 'blocked' : 'allowed';
	if (matchType === 'domain') {
		return (
			<>
				{verb} <D>{d}</D> and all subdomains like <D>www.{d}</D> and{' '}
				<D>mail.{d}</D>.
			</>
		);
	}
	if (matchType === 'wildcard') {
		return (
			<>
				{verb} subdomains like <D>www.{d}</D> and <D>mail.{d}</D>, but not{' '}
				<D>{d}</D> itself.
			</>
		);
	}
	return (
		<>
			{verb} <D>{d}</D> only. Subdomains like <D>www.{d}</D> are not {verb2}.
		</>
	);
}

export function AddRuleDialog({ onClose, onSubmit }: AddRuleDialogProps) {
	const [action, setAction] = useState<ListAction>('block');
	const [matchType, setMatchType] = useState<MatchType>('domain');

	const {
		register,
		handleSubmit,
		setError,
		control,
		formState: { errors, isSubmitting },
	} = useForm({ resolver: zodResolver(schema) });

	const domainValue = useWatch({ control, name: 'domain', defaultValue: '' });

	const onSubmitHandler = handleSubmit(async ({ domain }) => {
		try {
			const bare = domain.trim().toLowerCase().replace(/^\*\./, '');
			await onSubmit(bare, matchType, action);
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
									<ActionBadge
										action='block'
										onClick={() => setAction('block')}
										selected={action === 'block'}
									/>
									<ActionBadge
										action='allow'
										onClick={() => setAction('allow')}
										selected={action === 'allow'}
									/>
								</HStack>
							</Field.Root>

							<Field.Root invalid={!!errors.domain} mb='5'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Domain
								</Field.Label>
								<Input
									placeholder='e.g. ads.example.com'
									bg='bg.input'
									borderColor='border.input'
									_placeholder={{ color: 'fg.subtle' }}
									_hover={{ borderColor: 'accent.subtle' }}
									_focus={{ borderColor: 'accent.subtle' }}
									autoFocus
									{...register('domain')}
								/>
								{errors.domain?.message && (
									<Field.ErrorText color='status.error' fontSize='xs' mt='1'>
										{errors.domain.message}
									</Field.ErrorText>
								)}
							</Field.Root>

							<Field.Root mb='5'>
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
									{getMatchDescription(domainValue ?? '', matchType, action)}
								</Text>
							</Field.Root>

							<FormError message={errors.root?.message} />

							<Text color='fg.muted' fontSize='xs' mb='5'>
								{action === 'block'
									? 'Matching DNS queries will be denied.'
									: 'Matching domains will always resolve, bypassing any block rules.'}
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
									bg={
										action === 'block'
											? 'status.errorMuted'
											: 'status.successMuted'
									}
									color={action === 'block' ? 'status.error' : 'status.success'}
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
