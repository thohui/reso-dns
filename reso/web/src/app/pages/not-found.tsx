import { Box, Button, Heading, Icon, Text, VStack } from '@chakra-ui/react';
import { ArrowLeft } from 'lucide-react';
import { useNavigate } from 'react-router';

export default function NotFoundPage() {
	const navigate = useNavigate();

	return (
		<Box
			minH='100vh'
			display='flex'
			alignItems='center'
			justifyContent='center'
			px='4'
			position='relative'
		>
			<VStack gap='6' position='relative'>
				<VStack gap='2'>
					<Heading
						size='lg'
						fontWeight='600'
						letterSpacing='-0.02em'
						fontFamily='data'
					>
						404
					</Heading>
					<Text color='fg.muted' fontSize='sm'>
						This page does not exist.
					</Text>
				</VStack>
				<Button
					variant='ghost'
					color='fg.muted'
					_hover={{ bg: 'bg.subtle', color: 'fg' }}
					onClick={() => navigate('/home')}
					fontSize='sm'
					borderRadius='lg'
					transition='all 0.15s ease'
				>
					<Icon as={ArrowLeft} boxSize='4' mr='2' />
					Back to Dashboard
				</Button>
			</VStack>
		</Box>
	);
}
