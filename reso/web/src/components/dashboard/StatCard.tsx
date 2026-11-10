import { Box, Text } from '@chakra-ui/react';

interface Props {
	label: string;
	value: string | number;
	accentColor: string;
	isLoading?: boolean;
	compact?: boolean;
}

export function StatCard({
	label,
	value,
	accentColor,
	isLoading,
	compact,
}: Props) {
	return (
		<Box
			bg='bg.panel'
			borderRadius='xl'
			borderWidth='1px'
			borderColor='border'
			py={compact ? '3' : '5'}
			px={compact ? '4' : '5'}
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

			<Text
				color='fg.subtle'
				fontSize='xs'
				fontWeight='500'
				textTransform='uppercase'
				letterSpacing='0.05em'
				mb={compact ? '1' : '3'}
				truncate
			>
				{label}
			</Text>
			<Text
				fontSize={compact ? 'lg' : '2xl'}
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
