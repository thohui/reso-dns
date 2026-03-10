import {
	Box,
	Button,
	Flex,
	HStack,
	Icon,
	Text,
	VStack,
} from '@chakra-ui/react';
import {
	Ban,
	FileText,
	Globe,
	Home,
	LogOut,
	type LucideIcon,
	Settings,
} from 'lucide-react';
import { Suspense } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import Logo from '../assets/logo.svg?react';
import { PageLoader } from '../components/PageLoader';
import { useLogout } from '../hooks/useLogout';

interface MenuItem {
	path: string;
	label: string;
	icon?: LucideIcon;
}

const menuItems: MenuItem[] = [
	{ path: '/home', label: 'Home', icon: Home },
	{ path: '/blocklist', label: 'Blocklist', icon: Ban },
	{ path: '/local-records', label: 'Local Records', icon: Globe },
	{ path: '/logs', label: 'Logs', icon: FileText },
	{ path: '/config', label: 'Config', icon: Settings },
];

export function DashboardLayout() {
	const location = useLocation();
	const navigate = useNavigate();

	const logout = useLogout();

	const handleLogout = async () => {
		await logout.mutateAsync();
		navigate('/', { replace: true });
	};

	return (
		<Flex minH='100vh'>
			<Box
				w='240px'
				minW='240px'
				bg='bg.panel'
				borderRightWidth='1px'
				borderColor='border'
				py='5'
				px='3'
				display='flex'
				flexDirection='column'
			>
				<HStack gap='3' px='3' mb='8'>
					<Logo width={28} height={28} />
					<Text
						fontWeight='600'
						fontSize='sm'
						letterSpacing='-0.02em'
						fontFamily="'Mozilla Text', sans-serif"
					>
						[ResoDNS]
					</Text>
				</HStack>

				<VStack gap='0.5' align='stretch' flex='1'>
					{menuItems.map((item) => {
						const isActive = location.pathname === item.path;
						return (
							<Button
								key={item.path}
								variant='ghost'
								justifyContent='flex-start'
								bg={isActive ? 'accent.muted' : 'transparent'}
								color={isActive ? 'accent.fg' : 'fg.muted'}
								borderWidth='1px'
								borderColor={isActive ? 'accent.muted' : 'transparent'}
								_hover={{
									bg: isActive ? 'accent.muted' : 'bg.subtle',
									color: isActive ? 'accent.fg' : 'fg',
								}}
								onClick={() => navigate(item.path)}
								px='3'
								py='2'
								h='auto'
								fontSize='sm'
								fontWeight={isActive ? '500' : '400'}
								borderRadius='lg'
								transition='all 0.15s ease'
							>
								<Icon as={item.icon} boxSize='4' mr='3' />
								{item.label}
							</Button>
						);
					})}
				</VStack>

				<Box borderTopWidth='1px' borderColor='border' pt='3' mt='3'>
					<Button
						variant='ghost'
						justifyContent='flex-start'
						color='fg.faint'
						_hover={{ bg: 'bg.subtle', color: 'status.error' }}
						onClick={handleLogout}
						loading={logout.isPending}
						px='3'
						py='2'
						h='auto'
						fontSize='sm'
						fontWeight='400'
						borderRadius='lg'
						w='full'
						transition='all 0.15s ease'
					>
						<Icon as={LogOut} boxSize='4' mr='3' />
						Log out
					</Button>
				</Box>
			</Box>

			<Box flex='1' p='8' overflowY='auto' maxH='100vh'>
				<Box maxW='1400px' mx='auto'>
					<Suspense fallback={<PageLoader />}>
						<Box key={location.pathname} className='animate-fade-in'>
							<Outlet />
						</Box>
					</Suspense>
				</Box>
			</Box>
		</Flex>
	);
}
