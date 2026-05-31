import {
	Box,
	Button,
	Dialog,
	HStack,
	Heading,
	Icon,
	Text,
} from '@chakra-ui/react';
import { CheckCircle } from 'lucide-react';

interface ApiKeyCreatedDialogProps {
	keyId: string;
	onClose: () => void;
}

export function ApiKeyCreatedDialog({ keyId, onClose }: ApiKeyCreatedDialogProps) {
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
					<Dialog.Header pt='6' px='6' pb='0'>
						<HStack gap='3'>
							<Icon as={CheckCircle} boxSize='5' color='status.success' />
							<Heading size='md'>API Key Created</Heading>
						</HStack>
					</Dialog.Header>

					<Dialog.Body px='6' pb='0' pt='4'>
						<Text color='fg.muted' fontSize='sm' mb='3'>
							Copy your API key now. It won't be shown again.
						</Text>
						<Box
							bg='bg.subtle'
							borderWidth='1px'
							borderColor='border'
							borderRadius='md'
							px='4'
							py='3'
							mb='2'
						>
							<Text fontFamily='mono' fontSize='sm' wordBreak='break-all'>
								{keyId}
							</Text>
						</Box>
					</Dialog.Body>

					<Dialog.Footer px='6' pb='6' pt='4' justifyContent='flex-end'>
						<Button
							bg='accent'
							color='fg'
							_hover={{ bg: 'accent.hover' }}
							px='5'
							onClick={onClose}
						>
							Done
						</Button>
					</Dialog.Footer>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
	);
}
