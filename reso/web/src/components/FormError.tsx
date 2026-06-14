import { HStack, Icon, Text } from '@chakra-ui/react';
import { AlertCircle } from 'lucide-react';

interface FormErrorProps {
	message: string | undefined;
}

export function FormError({ message }: FormErrorProps) {
	if (!message) return null;

	return (
		<HStack
			gap='2'
			px='3'
			py='2'
			mb='4'
			bg='status.errorMuted'
			borderWidth='1px'
			borderColor='status.error'
			borderRadius='md'
		>
			<Icon as={AlertCircle} boxSize='4' color='status.error' flexShrink={0} />
			<Text fontSize='sm' color='status.error' lineHeight='1.5'>
				{message}
			</Text>
		</HStack>
	);
}
