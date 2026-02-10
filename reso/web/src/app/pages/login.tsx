import {
	Box,
	Button,
	Field,
	Heading,
	Icon,
	Input,
	Text,
	VStack,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { Shield } from 'lucide-react';

import { useForm } from 'react-hook-form';

import { useNavigate } from 'react-router-dom';
import z from 'zod';
import { useApiClient } from '../../contexts/ApiClientContext';

const loginSchema = z.object({
	username: z.string().min(1),
	password: z.string().min(1),
});

export default function LoginPage() {
	const navigate = useNavigate();

	const apiClient = useApiClient();

	const {
		register,
		handleSubmit,
		setError,
		formState: { errors, isLoading },
	} = useForm({ resolver: zodResolver(loginSchema) });

	const loginMutation = useMutation({
		mutationFn: async (data: z.infer<typeof loginSchema>) => {
			await apiClient.login(data.username, data.password);
		},
	});

	const onSubmit = handleSubmit(async (data) => {
		await loginMutation.mutateAsync(data, {
			onSuccess: () => navigate('/home'),
			onError: () => {
				setError('username', { message: 'Invalid username or password' });
				setError('password', { message: 'Invalid username or password' });
			},
		});
	});

	return (
		<Box
			minH='100vh'
			display='flex'
			alignItems='center'
			justifyContent='center'
			bg='gray.950'
			px='4'
		>
			<Box
				w='full'
				maxW='sm'
				bg='gray.900'
				borderRadius='xl'
				p='8'
				borderWidth='1px'
			>
				<VStack gap='2' mb='8'>
					<Box p='3' bg='green.600' borderRadius='xl' boxShadow='lg'>
						<Icon as={Shield} boxSize='8' color='white' />
					</Box>
					<Box textAlign='center' mt='2'>
						<Heading size='lg' color='white'>
							Reso
						</Heading>
						<Text color='gray.400' fontSize='sm' mt='1'>
							Network-wide DNS Protection
						</Text>
					</Box>
				</VStack>

				<form onSubmit={onSubmit}>
					<VStack gap='5'>
						<Field.Root w='full' invalid={!!errors.username}>
							<Field.Label color='gray.300' fontSize='sm'>
								Username
							</Field.Label>
							<Input
								type='text'
								placeholder='Enter your username'
								bg='gray.800'
								borderColor='gray.700'
								color='white'
								aria-errormessage={errors.username?.message}
								_placeholder={{ color: 'gray.500' }}
								_focus={{
									borderColor: 'green.500',
									boxShadow: '0 0 0 1px var(--chakra-colors-green-500)',
								}}
								{...register('username')}
							></Input>
							<Field.ErrorText>{errors.username?.message}</Field.ErrorText>
						</Field.Root>
						<Field.Root w='full' invalid={!!errors.password}>
							<Field.Label color='gray.300' fontSize='sm'>
								Password
							</Field.Label>
							<Input
								type='password'
								placeholder='Enter your password'
								bg='gray.800'
								borderColor='gray.700'
								color='white'
								aria-errormessage={errors.password?.message}
								_placeholder={{ color: 'gray.500' }}
								_focus={{
									borderColor: 'green.500',
									boxShadow: '0 0 0 1px var(--chakra-colors-green-500)',
								}}
								{...register('password')}
							/>
							<Field.ErrorText>{errors.password?.message}</Field.ErrorText>
						</Field.Root>

						<Button
							type='submit'
							w='full'
							bg='green.600'
							color='white'
							fontWeight='medium'
							_hover={{ bg: 'green.700' }}
							loading={isLoading}
							loadingText='Signing in...'
						>
							Sign In
						</Button>
					</VStack>
				</form>
			</Box>
		</Box>
	);
}
