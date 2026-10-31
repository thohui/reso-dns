import {
	Box,
	Button,
	Field,
	Group,
	Heading,
	Input,
	Text,
	VStack,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import z from 'zod';
import Logo from '@/assets/logo.svg?react';
import { useApiClient } from '@/contexts/ApiClientContext';
import { getVersion } from '@/lib/version';

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
			px='4'
			position='relative'
		>
			<Box
				w='full'
				maxW='380px'
				bg='bg.panel'
				borderRadius='2xl'
				p='8'
				borderWidth='1px'
				borderColor='border'
				position='relative'
				boxShadow='0 0 60px rgba(0, 0, 0, 0.4)'
				className='animate-slide-in-up'
			>
				<VStack gap='2' mb='8'>
					<Logo width={56} height={56} />
					<Box textAlign='center' mt='2'>
						<Heading
							size='lg'
							fontWeight='600'
							letterSpacing='-0.02em'
							fontFamily="'Mozilla Text', sans-serif"
						>
							[ResoDNS]
						</Heading>
					</Box>
				</VStack>

				<form onSubmit={onSubmit}>
					<VStack gap='5'>
						<Field.Root w='full' invalid={!!errors.username || !errors.root}>
							<Field.Label
								color='fg.subtle'
								fontSize='xs'
								fontWeight='500'
								textTransform='uppercase'
								letterSpacing='0.05em'
							>
								Username
							</Field.Label>
							<Group w='full'>
								<Input
									type='text'
									placeholder='Enter your username'
									bg='bg.input'
									borderColor='border.input'
									borderRadius='lg'
									_placeholder={{ color: 'fg.faint' }}
									_focus={{
										borderColor: 'accent',
										boxShadow: '0 0 0 1px rgba(233, 30, 120, 0.4)',
									}}
									transition='all 0.15s ease'
									{...register('username')}
								/>
							</Group>
							<Field.ErrorText>
								{errors.root?.message ?? errors.username?.message}
							</Field.ErrorText>
						</Field.Root>

						<Field.Root w='full' invalid={!!errors.root || !!errors.password}>
							<Field.Label
								color='fg.subtle'
								fontSize='xs'
								fontWeight='500'
								textTransform='uppercase'
								letterSpacing='0.05em'
							>
								Password
							</Field.Label>
							<Group w='full'>
								<Input
									type='password'
									placeholder='Enter your password'
									bg='bg.input'
									borderColor='border.input'
									borderRadius='lg'
									_placeholder={{ color: 'fg.faint' }}
									_focus={{
										borderColor: 'accent',
										boxShadow: '0 0 0 1px rgba(233, 30, 120, 0.4)',
									}}
									transition='all 0.15s ease'
									{...register('password')}
								/>
							</Group>
							<Field.ErrorText>
								{errors.root?.message ?? errors.password?.message}
							</Field.ErrorText>
						</Field.Root>

						<Button
							type='submit'
							w='full'
							bg='accent'
							color='fg'
							fontWeight='500'
							fontSize='sm'
							borderRadius='lg'
							_hover={{ bg: 'accent.hover' }}
							loading={isLoading}
							loadingText='Signing in...'
							transition='all 0.15s ease'
						>
							Sign In
						</Button>
						<Text fontSize='xs' color='fg.faint'>
							v{getVersion()}
						</Text>
					</VStack>
				</form>
			</Box>
		</Box>
	);
}
