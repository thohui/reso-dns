import {
	Box,
	HStack,
	Icon,
	IconButton,
	Input,
	Table,
	Text,
} from '@chakra-ui/react';
import {
	Ban,
	Globe,
	Search,
	ToggleLeft,
	ToggleRight,
	Trash2,
} from 'lucide-react';
import { useState } from 'react';
import type { BlockedDomain } from '../../lib/api/blocklist';

function formatTimeAgo(timestamp: number): string {
	const seconds = Math.floor((Date.now() - timestamp) / 1000);
	if (seconds < 60) return 'just now';
	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return `${minutes}m ago`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h ago`;
	const days = Math.floor(hours / 24);
	return `${days}d ago`;
}

interface BlocklistGridProps {
	blocklist: BlockedDomain[];
	onRemove: (domain: string) => void;
	onToggle: (domain: string) => void;
}

export function BlocklistGrid({
	blocklist,
	onRemove,
	onToggle,
}: BlocklistGridProps) {
	const [search, setSearch] = useState('');

	const filtered = blocklist.filter(
		(d) => !search || d.domain.toLowerCase().includes(search.toLowerCase()),
	);

	return (
		<Box>
			<Box position='relative' mb='4'>
				<Box
					position='absolute'
					left='3'
					top='50%'
					transform='translateY(-50%)'
					zIndex='1'
				>
					<Icon as={Search} boxSize='4' color='fg.subtle' />
				</Box>
				<Input
					placeholder='Search domains...'
					value={search}
					onChange={(e) => setSearch(e.target.value)}
					bg='bg.panel'
					borderColor='border.input'
					pl='10'
					_placeholder={{ color: 'fg.subtle' }}
					_focus={{ borderColor: 'accent.subtle' }}
					size='sm'
				/>
			</Box>

			<Box
				bg='bg.panel'
				borderRadius='lg'
				borderWidth='1px'
				borderColor='border'
				overflow='hidden'
			>
				{filtered.length === 0 ? (
					<Box py='14' textAlign='center'>
						<Icon
							as={search ? Search : Globe}
							boxSize='10'
							color='fg.subtle'
							mb='3'
							display='block'
							mx='auto'
						/>
						<Text color='fg.muted' fontSize='sm' mb='1'>
							{search
								? 'No domains match your search'
								: 'No domains blocked yet'}
						</Text>
						<Text color='fg.subtle' fontSize='xs'>
							{search
								? 'Try adjusting your search'
								: 'Click "Add Domain" to get started'}
						</Text>
					</Box>
				) : (
					<Box overflowX='auto'>
						<Table.Root size='sm'>
							<Table.Header>
								<Table.Row bg='bg.subtle' borderColor='border'>
									<Table.ColumnHeader
										color='fg.subtle'
										fontSize='xs'
										textTransform='uppercase'
										letterSpacing='0.05em'
										fontWeight='600'
										py='3'
										px='4'
									>
										Domain
									</Table.ColumnHeader>
									<Table.ColumnHeader
										color='fg.subtle'
										fontSize='xs'
										textTransform='uppercase'
										letterSpacing='0.05em'
										fontWeight='600'
										py='3'
										px='4'
										textAlign='right'
									>
										Added
									</Table.ColumnHeader>
									<Table.ColumnHeader
										color='fg.subtle'
										fontSize='xs'
										textTransform='uppercase'
										letterSpacing='0.05em'
										fontWeight='600'
										py='3'
										px='4'
										textAlign='center'
									>
										Status
									</Table.ColumnHeader>
									<Table.ColumnHeader py='3' px='4' w='10' />
								</Table.Row>
							</Table.Header>
							<Table.Body>
								{filtered.map((entry) => (
									<Table.Row
										key={entry.domain}
										bg='bg.panel'
										borderColor='border'
										_hover={{ bg: 'bg.subtle' }}
										transition='background 0.15s'
										opacity={entry.enabled ? 1 : 0.5}
									>
										<Table.Cell py='3.5' px='4'>
											<HStack gap='3'>
												<Icon
													as={Ban}
													boxSize='3.5'
													color={entry.enabled ? 'status.error' : 'fg.subtle'}
												/>
												<Text
													fontFamily="'Mozilla Text', sans-serif"
													fontSize='sm'
													fontWeight='500'
												>
													{entry.domain}
												</Text>
											</HStack>
										</Table.Cell>
										<Table.Cell py='3.5' px='4' textAlign='right'>
											<Text color='fg.muted' fontSize='sm'>
												{formatTimeAgo(entry.created_at)}
											</Text>
										</Table.Cell>
										<Table.Cell py='3.5' px='4' textAlign='center'>
											<IconButton
												aria-label={
													entry.enabled ? 'Disable domain' : 'Enable domain'
												}
												variant='plain'
												size='xs'
												color={entry.enabled ? 'status.success' : 'fg.subtle'}
												_hover={{ opacity: 0.8, bg: 'transparent' }}
												transition='all 0.15s'
												onClick={() => onToggle(entry.domain)}
											>
												<Icon
													as={entry.enabled ? ToggleRight : ToggleLeft}
													boxSize='5'
												/>
											</IconButton>
										</Table.Cell>
										<Table.Cell py='3.5' px='4'>
											<Box
												as='button'
												cursor='pointer'
												display='inline-flex'
												p='1'
												borderRadius='md'
												color='fg.subtle'
												_hover={{
													color: 'status.error',
													bg: 'status.errorMuted',
												}}
												transition='all 0.15s'
												onClick={() => onRemove(entry.domain)}
											>
												<Icon as={Trash2} boxSize='3.5' />
											</Box>
										</Table.Cell>
									</Table.Row>
								))}
							</Table.Body>
						</Table.Root>
					</Box>
				)}
			</Box>
		</Box>
	);
}
