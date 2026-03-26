import {
	Button,
	Dialog,
	Field,
	HStack,
	Heading,
	Icon,
	IconButton,
	Text,
} from '@chakra-ui/react';
import { AlertTriangle, Pencil, X } from 'lucide-react';
import { useState } from 'react';
import type { DomainRule, ListAction } from '@/lib/api/domain-rules';
import { ActionBadge } from '@/components/ActionBadge';

interface EditRuleDialogProps {
	rule: DomainRule;
	onClose: () => void;
	onSubmit: (domain: string, action: ListAction) => Promise<void>;
}

export function EditRuleDialog({
	rule,
	onClose,
	onSubmit,
}: EditRuleDialogProps) {
	const [action, setAction] = useState<ListAction>(rule.action);
	const [isSubmitting, setIsSubmitting] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		setIsSubmitting(true);
		setError(null);
		try {
			await onSubmit(rule.domain, action);
		} catch (e) {
			setError(e instanceof Error ? e.message : 'An error occurred');
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
									<Heading size='md'>Edit Rule</Heading>
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

						{rule.subscription_id && (
							<HStack
								gap='3'
								px='4'
								py='3'
								bg='status.warnMuted'
								borderBottomWidth='1px'
								borderColor='status.warn'
							>
								<Icon
									as={AlertTriangle}
									boxSize='4'
									color='status.warn'
									flexShrink={0}
								/>
								<Text fontSize='xs' color='status.warn' lineHeight='1.5'>
									This domain is managed by a subscription. Saving will detach
									it and prevent future syncs from overwriting your change.
								</Text>
							</HStack>
						)}

						<Dialog.Body px='6' pb='0' pt='4'>
							<Field.Root mb='5'>
								<Field.Label color='fg.muted' fontSize='sm'>
									Domain
								</Field.Label>
								<Text
									fontFamily="'Mozilla Text', sans-serif"
									fontSize='sm'
									fontWeight='500'
									color='fg'
									py='2'
									px='3'
									bg='bg.subtle'
									borderRadius='md'
									borderWidth='1px'
									borderColor='border'
								>
									{rule.domain}
								</Text>
							</Field.Root>

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

							{error && (
								<Text color='status.error' fontSize='xs' mb='4'>
									{error}
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
									disabled={action === rule.action && !rule.subscription_id}
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
