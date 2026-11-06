import {
	Box,
	Button,
	Drawer,
	Flex,
	HStack,
	Icon,
	Portal,
	Text,
	Tooltip,
	VStack,
} from '@chakra-ui/react';
import {
	Ban,
	FileText,
	Globe,
	Home,
	Key,
	LogOut,
	type LucideIcon,
	Menu,
	Monitor,
	Settings,
} from 'lucide-react';
import { Suspense, useState } from 'react';
import { NavLink, Outlet, useLocation, useNavigate } from 'react-router';
import Logo from '@/assets/logo.svg?react';
import { PageLoader } from '@/components/PageLoader';
import { useLogout } from '@/hooks/auth/useLogout';
import { useLiveStats } from '@/hooks/dashboard/useLiveStats';
import { useUptime } from '@/hooks/useUptime';
import { getVersion } from '@/lib/version';

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
	{ path: '/clients', label: 'Clients', icon: Monitor },
	{ path: '/api-keys', label: 'API keys', icon: Key },
	{ path: '/config', label: 'Config', icon: Settings },
];

function SidebarContent({ onNavigate }: { onNavigate?: () => void }) {
	const navigate = useNavigate();
	const logout = useLogout();
	const { data: liveData } = useLiveStats();
	const uptime = useUptime(liveData?.live_since);

	const handleLogout = async () => {
		await logout.mutateAsync();
		navigate('/', { replace: true });
	};

	return (
		<>
			<Tooltip.Root openDelay={300}>
				<Tooltip.Trigger asChild>
					<HStack gap='3' px='3' mb='8' align='baseline' w='fit-content'>
						<Logo width={28} height={28} style={{ alignSelf: 'center' }} />
						<Text
							fontWeight='600'
							fontSize='sm'
							letterSpacing='-0.02em'
							fontFamily="'Mozilla Text', sans-serif"
						>
							[ResoDNS]
						</Text>
						<Text fontSize='2xs' color='fg.faint' ml='-1.5'>
							v{getVersion()}
						</Text>
					</HStack>
				</Tooltip.Trigger>
				<Tooltip.Positioner>
					<Tooltip.Content
						fontSize='xs'
						bg='bg.panel'
						color='fg'
						borderWidth='1px'
						borderColor='border'
					>
						Uptime: {uptime.text}
					</Tooltip.Content>
				</Tooltip.Positioner>
			</Tooltip.Root>

			<VStack as='nav' aria-label='Main' gap='0.5' align='stretch' flex='1'>
				{menuItems.map((item) => (
					<Button
						key={item.path}
						asChild
						variant='ghost'
						justifyContent='flex-start'
						color='fg.muted'
						borderWidth='1px'
						borderColor='transparent'
						_hover={{ bg: 'bg.subtle', color: 'fg' }}
						_currentPage={{
							bg: 'accent.muted',
							color: 'accent.fg',
							borderColor: 'accent.muted',
							fontWeight: '500',
							_hover: { bg: 'accent.muted', color: 'accent.fg' },
						}}
						px='3'
						py='2'
						h='auto'
						fontSize='sm'
						fontWeight='400'
						borderRadius='lg'
						transition='all 0.15s ease'
					>
						<NavLink to={item.path} end onClick={onNavigate}>
							<Icon as={item.icon} boxSize='4' mr='3' />
							{item.label}
						</NavLink>
					</Button>
				))}
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
					color='fg.muted'
					size='sm'
					_hover={{ bg: 'bg.subtle' }}
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
				h={{ md: '100vh' }}
				display='flex'
				flexDir='column'
			>
				<Box
					maxW='1400px'
					mx='auto'
					w='full'
					flex='1'
					minH='0'
					display='flex'
					flexDir='column'
				>
					<Suspense fallback={<PageLoader />}>
						<Box
							key={location.pathname}
							className='animate-fade-in'
							flex='1'
							minH='0'
							display='flex'
							flexDir='column'
						>
							<Outlet />
						</Box>
					</Suspense>
				</Box>
			</Box>
		</Flex>
	);
}
