import { Box, Heading, Text } from '@chakra-ui/react';

export interface Props {
	title: string;
	description?: string;
	children: React.ReactNode;
}

export function ConfigSection({ title, description, children }: Props) {
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
				<Heading size='md' mb={1}>
					{title}
				</Heading>
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
