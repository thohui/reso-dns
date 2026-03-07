import {
	Box,
	Button,
	Field,
	Heading,
	HStack,
	Icon,
	IconButton,
	Input,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { ArrowLeft, Check, ChevronRight, Plus, X } from 'lucide-react';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import type { Upstream } from '../../lib/api/config';
import {
	detectProtocol,
	type ProviderGroup,
	providerGroups,
} from '../../lib/config/providers';
import { UpstreamSpecSchema } from '../../lib/config/schema';
import { hexToRgba } from '../../lib/theme';

export function UpstreamPicker({
	existingUpstreams,
	onAdd,
	onClose,
}: {
	existingUpstreams: Upstream[];
	onAdd: (upstream: Upstream) => void;
	onClose: () => void;
}) {
	const [view, setView] = useState<'providers' | 'servers' | 'custom'>(
		'providers',
	);

	const [selectedGroup, setSelectedGroup] = useState<ProviderGroup | null>(
		null,
	);

	const handleAddServer = (upstream: Upstream) => {
		if (!existingUpstreams.includes(upstream)) {
			onAdd(upstream);
		}
	};

	const handleSelect = (provider: ProviderGroup | 'custom') => {
		if (provider === 'custom') {
			setView('custom');
			return;
		}

		setSelectedGroup(provider);
		setView('servers');
	};

	const handleBack = () => {
		if (view === 'servers') {
			setView('providers');
			setSelectedGroup(null);
		} else if (view === 'custom') setView('providers');
	};

	return (
		<Box
			position='fixed'
			inset='0'
			zIndex='1000'
			display='flex'
			alignItems='center'
			justifyContent='center'
		>
			<Box
				position='absolute'
				inset='0'
				bg='blackAlpha.800'
				backdropFilter='blur(8px)'
				onClick={onClose}
			/>
			<Box
				position='relative'
				bg='bg.panel'
				borderColor='border'
				borderWidth='1px'
				maxW='540px'
				w='full'
				mx='4'
				borderRadius='xl'
				boxShadow='0 25px 50px -12px rgba(0, 0, 0, 0.5)'
				overflow='hidden'
			>
				<HStack
					justify='space-between'
					px='6'
					py='4'
					borderBottomWidth='1px'
					borderColor='border'
					bg='bg.subtle'
				>
					<HStack gap='3'>
						{view !== 'providers' && (
							<IconButton
								cursor='pointer'
								size='xs'
								variant='ghost'
								p='1'
								borderRadius='md'
								_hover={{ bg: 'bg.elevated' }}
								onClick={handleBack}
								display='flex'
								alignItems='center'
							>
								<Icon as={ArrowLeft} boxSize='4' color='fg.muted' />
							</IconButton>
						)}
						<Heading size='sm' fontWeight='500'>
							{view === 'providers'
								? 'Add Upstream Server'
								: view === 'servers' && selectedGroup
									? selectedGroup.name
									: 'Custom Server'}
						</Heading>
					</HStack>
					<IconButton
						cursor='pointer'
						variant='ghost'
						size='xs'
						p='1'
						borderRadius='md'
						_hover={{ bg: 'bg.elevated' }}
						onClick={onClose}
					>
						<Icon as={X} boxSize='4' color='fg.muted' />
					</IconButton>
				</HStack>

				<Box maxH='480px' overflowY='auto'>
					{view === 'providers' && (
						<ServersView
							existingUpstreams={existingUpstreams}
							handleSelectProvider={handleSelect}
						/>
					)}

					{selectedGroup && (
						<ProviderGroupView
							existingAddresses={existingUpstreams}
							selectedGroup={selectedGroup}
							onAdd={handleAddServer}
						/>
					)}

					{view === 'custom' && (
						<CustomView onClose={onClose} onAdd={handleAddServer} />
					)}
				</Box>
			</Box>
		</Box>
	);
}

export function ServersView({
	existingUpstreams,
	handleSelectProvider,
}: {
	existingUpstreams: Upstream[];
	handleSelectProvider: (provider: ProviderGroup | 'custom') => void;
}) {
	return (
		<Box>
			{providerGroups.map((group, i) => {
				const addedCount = group.servers.filter((s) =>
					existingUpstreams.includes(s.address),
				).length;
				const allAdded = addedCount === group.servers.length;
				return (
					<Box
						key={group.name}
						px='6'
						py='4'
						cursor={allAdded ? 'default' : 'pointer'}
						opacity={allAdded ? 0.4 : 1}
						_hover={allAdded ? {} : { bg: 'bg.subtle' }}
						borderBottomWidth={i < providerGroups.length - 1 ? '1px' : '0'}
						borderColor='border'
						transition='background 0.1s ease'
						onClick={() => !allAdded && handleSelectProvider(group)}
					>
						<HStack justify='space-between'>
							<HStack gap='4'>
								<Box
									w='10'
									h='10'
									borderRadius='lg'
									bg={hexToRgba(group.color, 0.1)}
									borderWidth='1px'
									borderColor={hexToRgba(group.color, 0.3)}
									display='flex'
									alignItems='center'
									justifyContent='center'
									fontWeight='bold'
									fontSize='xs'
									color={group.color}
									letterSpacing='-0.02em'
									flexShrink={0}
								>
									{group.slug}
								</Box>
								<Box>
									<Text fontSize='sm' fontWeight='500'>
										{group.name}
									</Text>
									<Text fontSize='xs' color='fg.muted' mt='0.5'>
										{group.description}
									</Text>
								</Box>
							</HStack>
							<HStack gap='3'>
								{addedCount > 0 && (
									<Text fontSize='xs' color='fg.faint'>
										{addedCount}/{group.servers.length} active
									</Text>
								)}
								{!allAdded && (
									<Icon as={ChevronRight} boxSize='4' color='fg.faint' />
								)}
								{allAdded && (
									<Text fontSize='xs' color='fg.faint'>
										All added
									</Text>
								)}
							</HStack>
						</HStack>
					</Box>
				);
			})}

			<Box
				px='6'
				py='4'
				cursor='pointer'
				_hover={{ bg: 'bg.subtle' }}
				onClick={() => handleSelectProvider('custom')}
			>
				<HStack justify='space-between'>
					<HStack gap='4'>
						<Box
							w='10'
							h='10'
							borderRadius='lg'
							bg='accent.muted'
							borderWidth='1px'
							borderColor='accent.bg'
							display='flex'
							alignItems='center'
							justifyContent='center'
							flexShrink={0}
						>
							<Icon as={Plus} boxSize='4' color='accent.fg' />
						</Box>
						<Box>
							<Text fontSize='sm' fontWeight='500'>
								Custom Server
							</Text>
							<Text fontSize='xs' color='fg.muted' mt='0.5'>
								Enter a custom DNS server address
							</Text>
						</Box>
					</HStack>
					<Icon as={ChevronRight} boxSize='4' color='fg.faint' />
				</HStack>
			</Box>
		</Box>
	);
}

function ProviderGroupView({
	selectedGroup,
	existingAddresses,
	onAdd,
}: {
	selectedGroup: ProviderGroup;
	existingAddresses: string[];
	onAdd: (upstream: Upstream) => void;
}) {
	return (
		<Box>
			{selectedGroup.servers.map((server, i) => {
				const isAdded = existingAddresses.includes(server.address);

				const protocol = detectProtocol(server.address);

				const protocolColor = PROTOCOL_COLORS[protocol] ?? '#71717a';

				const backgroundColor = hexToRgba(protocolColor, 0.1);

				return (
					<HStack
						key={server.address}
						px='6'
						py='4'
						justify='space-between'
						borderBottomWidth={
							i < selectedGroup.servers.length - 1 ? '1px' : '0'
						}
						borderColor='border'
						opacity={isAdded ? 0.4 : 1}
					>
						<Box>
							<HStack gap='2' mb='0.5'>
								<Text fontSize='sm' fontWeight='500'>
									{server.label}
								</Text>
								<Box
									px='1.5'
									py='0.5'
									borderRadius='md'
									bg={backgroundColor}
									borderWidth='1px'
									borderColor={protocolColor}
								>
									<Text
										fontSize='2xs'
										fontWeight='600'
										color={protocolColor}
										letterSpacing='0.02em'
									>
										{protocol}
									</Text>
								</Box>
							</HStack>
							<Text fontSize='xs' color='fg.muted' fontFamily='mono'>
								{server.address}
							</Text>
						</Box>
						{isAdded ? (
							<HStack gap='1.5' color='status.success'>
								<Icon as={Check} boxSize='3.5' />
								<Text fontSize='xs' fontWeight='500'>
									Active
								</Text>
							</HStack>
						) : (
							<Button
								size='sm'
								bg='accent'
								color='fg'
								_hover={{ bg: 'accent.hover' }}
								onClick={() => {
									onAdd(server.address);
								}}
								px='4'
								fontSize='xs'
								h='8'
								borderRadius='md'
							>
								Add
							</Button>
						)}
					</HStack>
				);
			})}
		</Box>
	);
}

const customViewSchema = z.object({
	upstream: UpstreamSpecSchema,
});

function CustomView({
	onClose,
	onAdd,
}: {
	onClose: () => void;
	onAdd: (upstream: Upstream) => void;
}) {
	const form = useForm({ resolver: zodResolver(customViewSchema) });

	const error = form.formState.errors.upstream;

	const onSubmit = form.handleSubmit(({ upstream }) => {
		onAdd(upstream);
		onClose();
	});

	return (
		<form onSubmit={onSubmit}>
			<Box px='6' py='5'>
				<Text color='fg.muted' fontSize='sm' mb='4' lineHeight='1.6'>
					Enter a DNS server address.
				</Text>

				<Box mb='4'>
					<Field.Root invalid={!!error} mb='6'>
						<Field.Label fontSize='sm' color='fg.muted' fontWeight='500' mb='2'>
							Server Address
						</Field.Label>
						<Input
							placeholder='e.g. 8.8.8.8'
							bg='bg.input'
							borderColor={error ? 'status.error' : 'border.input'}
							_placeholder={{ color: 'fg.subtle' }}
							_focus={{
								borderColor: error ? 'status.error' : 'accent.subtle',
							}}
							fontFamily='mono'
							fontSize='sm'
							{...form.register('upstream')}
						/>

						<Field.ErrorText>{error?.message}</Field.ErrorText>
					</Field.Root>
				</Box>

				<HStack justify='flex-end' gap='3'>
					<Button
						variant='ghost'
						color='fg.muted'
						_hover={{ bg: 'bg.subtle' }}
						onClick={onClose}
						px='4'
						fontSize='sm'
						h='9'
					>
						Cancel
					</Button>
					<Button
						bg='accent'
						color='fg'
						_hover={{ bg: 'accent.hover' }}
						px='5'
						type='submit'
						fontSize='sm'
						h='9'
					>
						Add Server
					</Button>
				</HStack>
			</Box>
		</form>
	);
}

export const PROTOCOL_COLORS: Record<string, string> = {
	'UDP/TCP': '#71717a',
	DoH: '#60a5fa',
	DoT: '#34d399',
};
