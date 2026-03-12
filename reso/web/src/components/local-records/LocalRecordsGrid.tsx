import { Box, Icon, IconButton, Input, Table, Text } from '@chakra-ui/react';
import { Globe, Search, ToggleLeft, ToggleRight, Trash2 } from 'lucide-react';
import { useState } from 'react';
import type { LocalRecord } from '../../lib/api/local-records';
import { recordTypeName } from '../../lib/dns';

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

const TYPE_COLORS: Record<number, string> = {
	1: '#60a5fa', // A
	28: '#a78bfa', // AAAA
	5: '#34d399', // CNAME
};

interface LocalRecordsGridProps {
	records: LocalRecord[];
	onRemove: (id: number) => void;
	onToggle: (id: number) => void;
}

export function LocalRecordsGrid({
	records,
	onRemove,
	onToggle,
}: LocalRecordsGridProps) {
	const [search, setSearch] = useState('');

	const filtered = records.filter(
		(r) =>
			!search ||
			r.name.toLowerCase().includes(search.toLowerCase()) ||
			r.value.toLowerCase().includes(search.toLowerCase()),
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
					placeholder='Search records...'
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
							{search ? 'No records match your search' : 'No local records yet'}
						</Text>
						<Text color='fg.subtle' fontSize='xs'>
							{search
								? 'Try adjusting your search'
								: 'Click "Add Record" to get started'}
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
										Name
									</Table.ColumnHeader>
									<Table.ColumnHeader
										color='fg.subtle'
										fontSize='xs'
										textTransform='uppercase'
										letterSpacing='0.05em'
										fontWeight='600'
										py='3'
										px='4'
									>
										Type
									</Table.ColumnHeader>
									<Table.ColumnHeader
										color='fg.subtle'
										fontSize='xs'
										textTransform='uppercase'
										letterSpacing='0.05em'
										fontWeight='600'
										py='3'
										px='4'
									>
										Value
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
										TTL
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
								{filtered.map((entry) => {
									const typeColor = TYPE_COLORS[entry.record_type] ?? '#71717a';
									return (
										<Table.Row
											key={entry.id}
											bg='bg.panel'
											borderColor='border'
											_hover={{ bg: 'bg.subtle' }}
											transition='background 0.15s'
											opacity={entry.enabled ? 1 : 0.5}
										>
											<Table.Cell py='3.5' px='4'>
												<Text fontFamily='mono' fontSize='sm' fontWeight='500'>
													{entry.name}
												</Text>
											</Table.Cell>
											<Table.Cell py='3.5' px='4'>
												<Box
													as='span'
													px='1.5'
													py='0.5'
													borderRadius='md'
													bg={`${typeColor}15`}
													borderWidth='1px'
													borderColor={`${typeColor}30`}
													fontSize='2xs'
													fontWeight='600'
													color={typeColor}
													letterSpacing='0.02em'
												>
													{recordTypeName(entry.record_type)}
												</Box>
											</Table.Cell>
											<Table.Cell py='3.5' px='4'>
												<Text fontFamily='mono' fontSize='sm' color='fg.muted'>
													{entry.value}
												</Text>
											</Table.Cell>
											<Table.Cell py='3.5' px='4' textAlign='right'>
												<Text color='fg.muted' fontSize='sm'>
													{entry.ttl}s
												</Text>
											</Table.Cell>
											<Table.Cell py='3.5' px='4' textAlign='right'>
												<Text color='fg.muted' fontSize='sm'>
													{formatTimeAgo(entry.created_at)}
												</Text>
											</Table.Cell>
											<Table.Cell py='3.5' px='4' textAlign='center'>
												<IconButton
													aria-label={
														entry.enabled ? 'Disable record' : 'Enable record'
													}
													variant='plain'
													size='xs'
													color={entry.enabled ? 'status.success' : 'fg.subtle'}
													_hover={{ opacity: 0.8, bg: 'transparent' }}
													transition='all 0.15s'
													onClick={() => onToggle(entry.id)}
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
													onClick={() => onRemove(entry.id)}
												>
													<Icon as={Trash2} boxSize='3.5' />
												</Box>
											</Table.Cell>
										</Table.Row>
									);
								})}
							</Table.Body>
						</Table.Root>
					</Box>
				)}
			</Box>
		</Box>
	);
}
