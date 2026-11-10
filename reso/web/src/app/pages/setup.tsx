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
import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import z from 'zod';
import Logo from '@/assets/logo.svg?react';
import { useApiClient } from '@/contexts/ApiClientContext';
import { useIsSetupRequired } from '@/hooks/auth/useIsSetupRequired';

const setupSchema = z
	.object({
		username: z.string().min(1, 'Username is required'),
		password: z.string().min(8, 'Password must be at least 8 characters'),
		confirmPassword: z.string().min(1, 'Please confirm your password'),
	})
	.refine((data) => data.password === data.confirmPassword, {
		message: 'Passwords do not match',
		path: ['confirmPassword'],
	});

export default function SetupPage() {
	const navigate = useNavigate();
	const apiClient = useApiClient();
	const setupRequired = useIsSetupRequired();

	useEffect(() => {
		if (!setupRequired) {
			navigate('/', { replace: true });
		}
	}, [setupRequired, navigate]);

	const {
		register,
		handleSubmit,
		setError,
		formState: { errors },
	} = useForm({ resolver: zodResolver(setupSchema) });

	const setupMutation = useMutation({
		mutationFn: async (data: { username: string; password: string }) => {
			await apiClient.setup(data.username, data.password);
		},
	});

	const onSubmit = handleSubmit(async (data) => {
		setupMutation.mutate(
			{ username: data.username, password: data.password },
			{
				onSuccess: () => navigate('/home'),
				onError: (e) => {
					setError('root', {
						message: e instanceof Error ? e.message : 'Setup failed',
					});
				},
			},
		);
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
			>
				<VStack gap='2' mb='8'>
					<Logo width={56} height={56} />
					<Box textAlign='center' mt='2'>
						<Heading
							size='lg'
							fontWeight='600'
							letterSpacing='-0.02em'
							fontFamily='data'
						>
							[ResoDNS]
						</Heading>
						<Text color='fg.muted' fontSize='sm' mt='1'>
							Create your administrator account
						</Text>
					</Box>
				</VStack>

				{errors.root?.message && (
					<Box
						bg='status.errorMuted'
						borderWidth='1px'
						borderColor='status.error'
						borderRadius='lg'
						px='4'
						py='3'
						mb='6'
					>
						<Text color='status.error' fontSize='sm'>
							{errors.root.message}
						</Text>
					</Box>
				)}

				<form onSubmit={onSubmit}>
					<VStack gap='5'>
						<Field.Root w='full' invalid={!!errors.username}>
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
									placeholder='Choose a username'
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
							<Field.ErrorText>{errors.username?.message}</Field.ErrorText>
						</Field.Root>

						<Field.Root w='full' invalid={!!errors.password}>
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
									placeholder='Minimum 8 characters'
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
							<Field.ErrorText>{errors.password?.message}</Field.ErrorText>
						</Field.Root>

						<Field.Root w='full' invalid={!!errors.confirmPassword}>
							<Field.Label
								color='fg.subtle'
								fontSize='xs'
								fontWeight='500'
								textTransform='uppercase'
								letterSpacing='0.05em'
							>
								Confirm Password
							</Field.Label>
							<Group w='full'>
								<Input
									type='password'
									placeholder='Repeat your password'
									bg='bg.input'
									borderColor='border.input'
									borderRadius='lg'
									_placeholder={{ color: 'fg.faint' }}
									_focus={{
										borderColor: 'accent',
										boxShadow: '0 0 0 1px rgba(233, 30, 120, 0.4)',
									}}
									transition='all 0.15s ease'
									{...register('confirmPassword')}
								/>
							</Group>
							<Field.ErrorText>
								{errors.confirmPassword?.message}
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
							loading={setupMutation.isPending}
							loadingText='Creating account...'
							transition='all 0.15s ease'
						>
							Create Account
						</Button>
					</VStack>
				</form>
			</Box>
		</Box>
	);
}
