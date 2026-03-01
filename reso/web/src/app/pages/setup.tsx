import {
	Box,
	Button,
	Field,
	Group,
	Heading,
	Icon,
	Input,
	Text,
	VStack,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { Lock, ShieldCheck, User } from 'lucide-react';
import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import z from 'zod';
import Logo from '../../assets/logo.svg?react';
import { useApiClient } from '../../contexts/ApiClientContext';
import { useIsSetupRequired } from '../../hooks/useIsSetupRequired';

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

	console.log('yo');
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
		await setupMutation.mutateAsync(
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
							fontFamily="'JetBrains Mono', monospace"
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
							<Group w='full' position='relative'>
								<Box
									position='absolute'
									left='3'
									top='50%'
									transform='translateY(-50%)'
									zIndex='1'
									pointerEvents='none'
								>
									<Icon as={User} boxSize='4' color='fg.faint' />
								</Box>
								<Input
									type='text'
									placeholder='Choose a username'
									pl='10'
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
							<Group w='full' position='relative'>
								<Box
									position='absolute'
									left='3'
									top='50%'
									transform='translateY(-50%)'
									zIndex='1'
									pointerEvents='none'
								>
									<Icon as={Lock} boxSize='4' color='fg.faint' />
								</Box>
								<Input
									type='password'
									placeholder='Minimum 8 characters'
									pl='10'
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
							<Group w='full' position='relative'>
								<Box
									position='absolute'
									left='3'
									top='50%'
									transform='translateY(-50%)'
									zIndex='1'
									pointerEvents='none'
								>
									<Icon as={ShieldCheck} boxSize='4' color='fg.faint' />
								</Box>
								<Input
									type='password'
									placeholder='Repeat your password'
									pl='10'
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
				<Text color='fg.faint' fontSize='xs' textAlign='center' mt='6'>
					Initial setup
				</Text>
			</Box>
		</Box>
	);
}
