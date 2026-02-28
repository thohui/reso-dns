import { Box, HStack, Text } from '@chakra-ui/react';

export function ConfigField({
	label,
	description,
	children,
}: {
	label: string;
	description?: string;
	children: React.ReactNode;
}) {
	return (
		<HStack justify='space-between' align='flex-start' py='3'>
			<Box flex='1' mr='8'>
				<Text fontSize='sm' fontWeight='medium'>
					{label}
				</Text>
				{description && (
					<Text color='fg.muted' fontSize='xs' mt='0.5'>
						{description}
					</Text>
				)}
			</Box>
			<Box minW='200px' maxW='280px' w='full'>
				{children}
			</Box>
		</HStack>
	);
}
