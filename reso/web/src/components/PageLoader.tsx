import { Text, VStack } from '@chakra-ui/react';
import Logo from '@/assets/logo.svg?react';

export function PageLoader() {
	return (
		<VStack justify='center' align='center' flex='1' minH='100vh' gap='5'>
			<Logo width={48} height={48} />
			<Text
				fontSize='sm'
				color='fg.faint'
				fontFamily="'Mozilla Text', sans-serif"
				letterSpacing='0.05em'
			>
				Loading...
			</Text>
		</VStack>
	);
}
