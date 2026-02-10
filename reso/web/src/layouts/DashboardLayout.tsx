import {
	Box,
	Button,
	Flex,
	HStack,
	Icon,
	Text,
	VStack,
} from '@chakra-ui/react';
import { Ban, FileText, Home, LogOut, Shield } from 'lucide-react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import { useLogout } from '../hooks/useLogout';

const menuItems = [
	{ path: '/home', label: 'Home', icon: Home },
	{ path: '/blocklist', label: 'Blocklist', icon: Ban },
	{ path: '/logs', label: 'Logs', icon: FileText },
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
		<Flex minH="100vh" bg="gray.950">
			<Box
				w="64"
				bg="gray.900"
				borderRightWidth="1px"
				borderColor="gray.800"
				p="4"
			>
				<VStack gap="6" align="stretch" h="full">
					<HStack gap="3" px="2" py="4">
						<Box p="2" bg="green.600" borderRadius="lg">
							<Icon as={Shield} boxSize="6" color="white" />
						</Box>
						<Box>
							<Text fontWeight="bold" color="white">
								Reso
							</Text>
							<Text fontSize="xs" color="gray.500">
								Admin Panel
							</Text>
						</Box>
					</HStack>

					<VStack gap="1" align="stretch" flex="1">
						{menuItems.map((item) => {
							const isActive = location.pathname === item.path;
							return (
								<Button
									key={item.path}
									variant={isActive ? 'solid' : 'ghost'}
									justifyContent="flex-start"
									bg={isActive ? 'green.600' : 'transparent'}
									color={isActive ? 'white' : 'gray.400'}
									_hover={{
										bg: isActive ? 'green.700' : 'gray.800',
										color: 'white',
									}}
									onClick={() => navigate(item.path)}
									px="4"
									py="3"
								>
									<Icon as={item.icon} boxSize="5" mr="3" />
									{item.label}
								</Button>
							);
						})}
					</VStack>

					<Button
						variant="ghost"
						justifyContent="flex-start"
						color="gray.400"
						_hover={{ bg: 'gray.800', color: 'white' }}
						onClick={handleLogout}
						px="4"
						py="3"
						loading={logout.isPending}
					>
						<Icon as={LogOut} boxSize="5" mr="3" />
						Logout
					</Button>
				</VStack>
			</Box>

			<Box flex="1" p="8" overflowY="auto">
				<Outlet />
			</Box>
		</Flex>
	);
}
