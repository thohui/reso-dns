import {
	Box,
	Button,
	Drawer,
	Flex,
	HStack,
	Icon,
	Portal,
	Text,
	VStack,
} from '@chakra-ui/react';
import {
	Ban,
	FileText,
	Globe,
	Home,
	LogOut,
	Menu,
	type LucideIcon,
	Settings,
} from 'lucide-react';
import { Suspense, useState } from 'react';
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
	{ path: '/domain-rules', label: 'Domain Rules', icon: Ban },
	{ path: '/local-records', label: 'Local Records', icon: Globe },
	{ path: '/logs', label: 'Logs', icon: FileText },
	{ path: '/config', label: 'Config', icon: Settings },
];

function SidebarContent({ onNavigate }: { onNavigate?: () => void }) {
	const location = useLocation();
	const navigate = useNavigate();
	const logout = useLogout();

	const handleNavigate = (path: string) => {
		navigate(path);
		onNavigate?.();
	};

	const handleLogout = async () => {
		await logout.mutateAsync();
		navigate('/', { replace: true });
	};

	return (
		<>
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
							onClick={() => handleNavigate(item.path)}
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
		</>
	);
}

export function DashboardLayout() {
	const location = useLocation();
	const [drawerOpen, setDrawerOpen] = useState(false);

	return (
		<Flex minH='100vh' direction={{ base: 'column', md: 'row' }}>
			<HStack
				display={{ base: 'flex', md: 'none' }}
				bg='bg.panel'
				borderBottomWidth='1px'
				borderColor='border'
				px='4'
				py='3'
				gap='3'
				position='sticky'
				top='0'
				zIndex='sticky'
			>
				<Button
					aria-label='Open navigation'
					variant='ghost'
					size='sm'
					px='2'
					onClick={() => setDrawerOpen(true)}
				>
					<Icon as={Menu} boxSize='5' />
				</Button>
				<HStack gap='2'>
					<Logo width={22} height={22} />
					<Text
						fontWeight='600'
						fontSize='sm'
						letterSpacing='-0.02em'
						fontFamily="'Mozilla Text', sans-serif"
					>
						[ResoDNS]
					</Text>
				</HStack>
			</HStack>

			<Drawer.Root
				open={drawerOpen}
				onOpenChange={(e) => setDrawerOpen(e.open)}
				placement='start'
			>
				<Portal>
					<Drawer.Backdrop />
					<Drawer.Positioner>
						<Drawer.Content
							bg='bg.panel'
							borderColor='border'
							borderRightWidth='1px'
							w='240px'
							maxW='80vw'
						>
							<Drawer.Body py='5' px='3' display='flex' flexDirection='column'>
								<SidebarContent onNavigate={() => setDrawerOpen(false)} />
							</Drawer.Body>
						</Drawer.Content>
					</Drawer.Positioner>
				</Portal>
			</Drawer.Root>

			<Box
				display={{ base: 'none', md: 'flex' }}
				w='240px'
				minW='240px'
				bg='bg.panel'
				borderRightWidth='1px'
				borderColor='border'
				py='5'
				px='3'
				flexDirection='column'
			>
				<SidebarContent />
			</Box>

			<Box
				id='main-scroll'
				flex='1'
				p={{ base: '4', md: '8' }}
				overflowY='auto'
				maxH={{ base: 'auto', md: '100vh' }}
			>

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
