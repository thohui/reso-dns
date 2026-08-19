import {
	Box,
	Button,
	Dialog,
	Field,
	Heading,
	HStack,
	Icon,
	IconButton,
	Input,
	NativeSelect,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { ArrowLeft, Check, ChevronRight, X } from 'lucide-react';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import type { Upstream, UpstreamKind } from '@/lib/api/config';
import {
	type DetectedProtocol,
	hasUpstream,
	normalizeEndpoint,
	type ProviderGroup,
	protocolForKind,
	providerGroups,
	serverUpstream,
	upstreamKey,
} from '@/lib/config/providers';
import {
	endpointInputSchema,
	tlsHostnameInputSchema,
} from '@/lib/config/schema';
import { hexToRgba } from '@/lib/theme';

interface Props {
	existingUpstreams: Upstream[];
	onAdd: (upstream: Upstream) => void;
	onClose: () => void;
}

export function UpstreamPicker({ existingUpstreams, onAdd, onClose }: Props) {
	const [view, setView] = useState<'providers' | 'servers' | 'custom'>(
		'providers',
	);

	const [selectedGroup, setSelectedGroup] = useState<ProviderGroup | null>(
		null,
	);

	const handleAddServer = (upstream: Upstream) => {
		if (!hasUpstream(existingUpstreams, upstream)) {
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

	let title = 'Custom Server';
	if (view === 'providers') title = 'Add Upstream Server';
	else if (view === 'servers' && selectedGroup) title = selectedGroup.name;

	return (
		<Dialog.Root open onOpenChange={({ open }) => !open && onClose()}>
			<Dialog.Backdrop backdropFilter='blur(8px)' bg='blackAlpha.800' />
			<Dialog.Positioner>
				<Dialog.Content
					bg='bg.panel'
					borderColor='border'
					borderWidth='1px'
					maxW='540px'
					borderRadius='xl'
					boxShadow='0 25px 50px -12px rgba(0, 0, 0, 0.5)'
					overflow='hidden'
				>
					<Dialog.Header
						px='6'
						py='4'
						borderBottomWidth='1px'
						borderColor='border'
						bg='bg.subtle'
					>
						<HStack justify='space-between' w='full'>
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
										aria-label='Go back'
									>
										<Icon as={ArrowLeft} boxSize='4' color='fg.muted' />
									</IconButton>
								)}
								<Heading size='sm' fontWeight='500'>
									{title}
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
								aria-label='Close dialog'
							>
								<Icon as={X} boxSize='4' color='fg.muted' />
							</IconButton>
						</HStack>
					</Dialog.Header>

					<Dialog.Body p='0' maxH='480px' overflowY='auto'>
						{view === 'providers' && (
							<ServersView
								existingUpstreams={existingUpstreams}
								handleSelectProvider={handleSelect}
							/>
						)}

						{selectedGroup && (
							<ProviderGroupView
								existingUpstreams={existingUpstreams}
								selectedGroup={selectedGroup}
								onAdd={handleAddServer}
							/>
						)}

						{view === 'custom' && (
							<CustomView onClose={onClose} onAdd={handleAddServer} />
						)}
					</Dialog.Body>
				</Dialog.Content>
			</Dialog.Positioner>
		</Dialog.Root>
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
					hasUpstream(existingUpstreams, serverUpstream(s)),
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
							<Box>
								<Text fontSize='sm' fontWeight='500'>
									{group.name}
								</Text>
								<Text fontSize='xs' color='fg.muted' mt='0.5'>
									{group.description}
								</Text>
							</Box>
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
	existingUpstreams,
	onAdd,
}: {
	selectedGroup: ProviderGroup;
	existingUpstreams: Upstream[];
	onAdd: (upstream: Upstream) => void;
}) {
	return (
		<Box>
			{selectedGroup.servers.map((server, i) => {
				const upstream = serverUpstream(server);
				const isAdded = hasUpstream(existingUpstreams, upstream);

				const protocol = protocolForKind(upstream.kind);

				const protocolColor = PROTOCOL_COLORS[protocol] ?? '#71717a';

				const backgroundColor = hexToRgba(protocolColor, 0.1);

				return (
					<HStack
						key={upstreamKey(upstream)}
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
								{server.hostname && (
									<Text as='span' ml='2' color='fg.subtle'>
										{server.hostname}
									</Text>
								)}
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
									onAdd(upstream);
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
	kind: z.enum(['plain', 'tls']),
	address: endpointInputSchema,
	hostname: tlsHostnameInputSchema,
});

const PROTOCOL_OPTIONS: { kind: UpstreamKind; label: string }[] = [
	{ kind: 'plain', label: 'UDP/TCP' },
	{ kind: 'tls', label: 'DoT' },
];

function CustomView({
	onClose,
	onAdd,
}: {
	onClose: () => void;
	onAdd: (upstream: Upstream) => void;
}) {
	const form = useForm({
		resolver: zodResolver(customViewSchema),
		defaultValues: { kind: 'plain' as UpstreamKind, address: '', hostname: '' },
	});

	const error = form.formState.errors.address;
	const hostnameError = form.formState.errors.hostname;
	const kind = form.watch('kind');

	const onSubmit = form.handleSubmit(({ kind, address, hostname }) => {
		const endpoint = normalizeEndpoint(address, kind);
		onAdd(
			kind === 'tls' && hostname
				? { kind, endpoint, hostname }
				: { kind, endpoint },
		);
		onClose();
	});

	return (
		<form onSubmit={onSubmit}>
			<Box px='6' py='5'>
				<Text color='fg.muted' fontSize='sm' mb='4' lineHeight='1.6'>
					Enter a DNS server IP address. The port is optional and defaults per
					protocol.
					{kind === 'tls' &&
						'The hostname is the name the certificate must match, not an address.'}
				</Text>

				<Box mb='4'>
					<Field.Root invalid={!!error} mb='6'>
						<Field.Label fontSize='sm' color='fg.muted' fontWeight='500' mb='2'>
							Server Address
						</Field.Label>
						<HStack gap='2' align='stretch'>
							<NativeSelect.Root w='auto' minW='120px'>
								<NativeSelect.Field
									bg='bg.input'
									borderColor='border.input'
									fontSize='sm'
									_hover={{ borderColor: 'accent.subtle' }}
									{...form.register('kind')}
								>
									{PROTOCOL_OPTIONS.map((o) => (
										<option key={o.kind} value={o.kind}>
											{o.label}
										</option>
									))}
								</NativeSelect.Field>
								<NativeSelect.Indicator />
							</NativeSelect.Root>
							<Input
								placeholder='8.8.8.8'
								bg='bg.input'
								borderColor={error ? 'status.error' : 'border.input'}
								_placeholder={{ color: 'fg.subtle' }}
								_hover={{
									borderColor: error ? 'status.error' : 'accent.subtle',
								}}
								_focus={{
									borderColor: error ? 'status.error' : 'accent.subtle',
								}}
								fontFamily='mono'
								fontSize='sm'
								flex='1'
								{...form.register('address')}
							/>
						</HStack>
						<Field.ErrorText>{error?.message}</Field.ErrorText>
					</Field.Root>

					{kind === 'tls' && (
						<Field.Root invalid={!!hostnameError} mb='6'>
							<Field.Label
								fontSize='sm'
								color='fg.muted'
								fontWeight='500'
								mb='2'
							>
								Certificate Hostname
								<Text as='span' ml='2' fontSize='xs' color='fg.subtle'>
									optional
								</Text>
							</Field.Label>
							<Input
								placeholder='dns.quad9.net'
								bg='bg.input'
								borderColor={hostnameError ? 'status.error' : 'border.input'}
								_placeholder={{ color: 'fg.subtle' }}
								_hover={{
									borderColor: hostnameError ? 'status.error' : 'accent.subtle',
								}}
								_focus={{
									borderColor: hostnameError ? 'status.error' : 'accent.subtle',
								}}
								fontFamily='mono'
								fontSize='sm'
								{...form.register('hostname')}
							/>
							<Field.ErrorText>{hostnameError?.message}</Field.ErrorText>
						</Field.Root>
					)}
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

export const PROTOCOL_COLORS: Record<DetectedProtocol, string> = {
	'UDP/TCP': '#71717a',
	DoH: '#60a5fa',
	DoT: '#34d399',
};
