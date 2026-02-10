import { Box, HStack, Icon, Text } from '@chakra-ui/react';
import type { Activity } from 'lucide-react';

const colorMap = {
	green: { bg: 'green.600' },
	red: { bg: 'red.600' },
	blue: { bg: 'blue.600' },
	yellow: { bg: 'yellow.600' },
} as const;

interface Props {
	label: string;
	value: string | number;
	icon: typeof Activity;
	color?: 'green' | 'red' | 'blue' | 'yellow';
}

export function StatCard({ label, value, icon, color = 'green' }: Props) {
	return (
		<Box
			bg='gray.900'
			borderRadius='lg'
			borderWidth='1px'
			borderColor='gray.800'
			p='6'
		>
			<HStack justify='space-between' mb='4'>
				<Text color='gray.400' fontSize='sm'>
					{label}
				</Text>
				<Box p='2' bg={colorMap[color].bg} borderRadius='md'>
					<Icon as={icon} boxSize='4' color='white' />
				</Box>
			</HStack>
			<Text color='white' fontSize='3xl' fontWeight='bold'>
				{value}
			</Text>
		</Box>
	);
}
