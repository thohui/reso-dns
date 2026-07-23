import { Box, HStack, Icon, Text } from '@chakra-ui/react';
import type { Activity } from 'lucide-react';

interface Props {
	label: string;
	value: string | number;
	icon: typeof Activity;
	accentColor: string;
	isLoading?: boolean;
}

export function StatCard({
	label,
	value,
	icon,
	accentColor,
	isLoading,
}: Props) {
	return (
		<Box
			bg='bg.panel'
			borderRadius='xl'
			borderWidth='1px'
			borderColor='border'
			p='5'
			position='relative'
			overflow='hidden'
			transition='border-color 0.2s ease'
			_hover={{ borderColor: 'border.input' }}
		>
			<Box
				position='absolute'
				top='0'
				left='0'
				right='0'
				h='1px'
				bg={accentColor}
				opacity='0.5'
			/>

			<HStack justify='space-between' mb='3' gap='2'>
				<Text
					color='fg.subtle'
					fontSize='xs'
					fontWeight='500'
					textTransform='uppercase'
					letterSpacing='0.05em'
					flex='1'
					minW='0'
					truncate
				>
					{label}
				</Text>
				<Icon as={icon} boxSize='4' color='fg.faint' flexShrink={0} />
			</HStack>
			<Text
				fontSize='2xl'
				fontWeight='600'
				letterSpacing='-0.02em'
				opacity={isLoading ? 0.3 : 1}
				transition='opacity 0.2s'
			>
				{value}
			</Text>
		</Box>
	);
}
