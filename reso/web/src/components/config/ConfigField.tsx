import { Box, Flex, Text } from '@chakra-ui/react';

export interface Props {
	label: string;
	description?: string;
	align?: 'flex-start' | 'center';
	children: React.ReactNode;
}

export function ConfigField({
	label,
	description,
	children,
	align = 'flex-start',
}: Props) {
	return (
		<Flex
			direction={{ base: 'column', sm: 'row' }}
			justify='space-between'
			align={{ base: 'flex-start', sm: align }}
			gap={{ base: '2', sm: '0' }}
			py='3'
		>
			<Box flex='1' mr={{ base: '0', sm: '8' }}>
				<Text fontSize='sm' fontWeight='medium'>
					{label}
				</Text>
				{description && (
					<Text color='fg.muted' fontSize='xs' mt='0.5'>
						{description}
					</Text>
				)}
			</Box>
			<Box
				minW={{ base: '0', sm: '200px' }}
				maxW={{ base: 'full', sm: '280px' }}
				w='full'
			>
				{children}
			</Box>
		</Flex>
	);
}
