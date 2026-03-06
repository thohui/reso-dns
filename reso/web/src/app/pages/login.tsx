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
import { Lock, User } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import z from 'zod';
import Logo from '../../assets/logo.svg?react';
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
						<Text color='fg.muted' fontSize='sm' mt='1'>
							Network-wide DNS Protection
						</Text>
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
									placeholder='Enter your username'
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
									placeholder='Enter your password'
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
					</VStack>
				</form>
				<Text color='fg.faint' fontSize='xs' textAlign='center' mt='6'>
					Admin panel
				</Text>
			</Box>
		</Box>
	);
}
