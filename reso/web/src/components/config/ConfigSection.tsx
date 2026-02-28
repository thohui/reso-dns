import { Box, Heading, HStack, Icon, Text } from '@chakra-ui/react';
import { type LucideIcon } from 'lucide-react';

export interface Props {
	title: string;
	description?: string;
	icon: LucideIcon;
	children: React.ReactNode;
}

export function ConfigSection({ title, description, icon, children }: Props) {
	return (
		<Box
			bg='bg.panel'
			borderRadius='xl'
			borderWidth='1px'
			borderColor='border'
			overflow='hidden'
			mb='6'
		>
			<Box px={6} pt={6} pb={4}>
				<HStack gap={3} mb={1}>
					<Icon as={icon} boxSize={5} color='accent.fg' />
					<Heading size='md'>{title}</Heading>
				</HStack>
				{description && (
					<Text color='fg.muted' fontSize='sm'>
						{description}
					</Text>
				)}
			</Box>
			<Box px={6} pb={6}>
				{children}
			</Box>
		</Box>
	);
}
