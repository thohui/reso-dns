import { Box, Button, Heading, Icon, Text, VStack } from '@chakra-ui/react';
import { ArrowLeft, SearchX } from 'lucide-react';
import { useNavigate } from 'react-router-dom';

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
			<Box
				position='absolute'
				top='50%'
				left='50%'
				transform='translate(-50%, -50%)'
				w='600px'
				h='600px'
				borderRadius='full'
				bg='radial-gradient(circle, rgba(233, 30, 120, 0.06) 0%, transparent 70%)'
				pointerEvents='none'
			/>

			<VStack gap='6' position='relative'>
				<Icon as={SearchX} boxSize='16' color='accent.fg' />
				<VStack gap='2'>
					<Heading
						size='lg'
						fontWeight='600'
						letterSpacing='-0.02em'
						fontFamily="'Mozilla Text', sans-serif"
					>
						404
					</Heading>
					<Text color='fg.muted' fontSize='sm'>
						This page doesn't exist.
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
