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
	createColumnHelper,
	flexRender,
	getCoreRowModel,
	useReactTable,
} from '@tanstack/react-table';
import { Monitor, Pencil, Search } from 'lucide-react';
import React, { useMemo } from 'react';
import { GridPage } from '@/components/GridPage';
import type { Client } from '@/lib/api/clients';
import { normalizeHostname } from '@/lib/clients';
import { formatTimeAgo } from '@/lib/time';

const columnHelper = createColumnHelper<Client>();

interface ClientsGridProps {
	clients: Client[];
	onEdit: (client: Client) => void;
	page: number;
	totalPages: number | null;
	total?: number | null;
	hasMore?: boolean;
	isLoading?: boolean;
	onPageChange: (page: number) => void;
	search: string;
	onSearchChange: (value: string) => void;
}

export function ClientsGrid({
	clients,
	onEdit,
	page,
	totalPages,
	total,
	hasMore,
	isLoading,
	onPageChange,
	search,
	onSearchChange,
}: ClientsGridProps) {
	const columns = useMemo(
		() => [
			columnHelper.display({
				id: 'name',
				header: 'Name',
				cell: ({ row }) => {
					const { display_name, hostname } = row.original;
					const name = display_name ?? normalizeHostname(hostname);
					return (
						<Table.Cell py='3' px='4'>
							<Text
								fontSize='sm'
								fontWeight='500'
								color={name ? 'fg' : 'fg.subtle'}
								truncate
							>
								{name ?? 'Unnamed'}
								{display_name && hostname && (
									<Text
										as='span'
										ml='2'
										fontFamily='data'
										fontSize='xs'
										fontWeight='400'
										color='fg.subtle'
									>
										{normalizeHostname(hostname)}
									</Text>
								)}
							</Text>
						</Table.Cell>
					);
				},
			}),
			columnHelper.accessor('ip', {
				header: 'IP Address',
				cell: ({ getValue }) => (
					<Table.Cell py='3' px='4'>
						<Text fontFamily='data' fontSize='sm' color='fg.muted'>
							{getValue()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('first_seen_at', {
				header: 'First Seen',
				cell: ({ getValue }) => (
					<Table.Cell py='3' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{formatTimeAgo(getValue())}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('last_seen_at', {
				header: 'Last Seen',
				cell: ({ getValue }) => (
					<Table.Cell py='3' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{formatTimeAgo(getValue())}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.display({
				id: 'edit',
				header: '',
				cell: ({ row }) => (
					<Table.Cell py='3' px='4'>
						<IconButton
							aria-label='Edit client'
							variant='ghost'
							size='2xs'
							p='1'
							borderRadius='md'
							color='fg.subtle'
							_hover={{ color: 'accent.fg', bg: 'accent.muted' }}
							transition='all 0.15s'
							onClick={() => onEdit(row.original)}
						>
							<Icon as={Pencil} boxSize='3.5' />
						</IconButton>
					</Table.Cell>
				),
			}),
		],
		[onEdit],
	);

	const table = useReactTable({
		data: clients,
		columns,
		getCoreRowModel: getCoreRowModel(),
	});

	const toolbar = (
		<HStack gap='3'>
			<Box position='relative' flex='1'>
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
					placeholder='Search clients...'
					value={search}
					onChange={(e) => onSearchChange(e.target.value)}
					bg='bg.panel'
					borderColor='border.input'
					pl='10'
					_placeholder={{ color: 'fg.subtle' }}
					_hover={{ borderColor: 'accent.subtle' }}
					_focus={{ borderColor: 'accent.subtle' }}
					size='sm'
				/>
			</Box>
		</HStack>
	);

	return (
		<GridPage
			toolbar={toolbar}
			isEmpty={clients.length === 0}
			emptyIcon={search ? Search : Monitor}
			emptyTitle={
				search ? 'No clients match your search' : 'No clients seen yet'
			}
			emptySubtitle={
				search
					? 'Try adjusting your search'
					: 'Clients show up here once they send a DNS query'
			}
			page={page}
			totalPages={totalPages}
			total={total}
			totalLabel='clients'
			hasMore={hasMore}
			isLoading={isLoading}
			onPageChange={onPageChange}
		>
			<Table.Root size='sm'>
				<Table.Header position='sticky' top='0' zIndex='1'>
					{table.getHeaderGroups().map((headerGroup) => (
						<Table.Row key={headerGroup.id} bg='bg.subtle' borderColor='border'>
							{headerGroup.headers.map((header) => (
								<Table.ColumnHeader
									key={header.id}
									py='3'
									px='4'
									color='fg.subtle'
									fontSize='xs'
									textTransform='uppercase'
									letterSpacing='0.05em'
									fontWeight='600'
									textAlign={
										header.id === 'first_seen_at' ||
										header.id === 'last_seen_at'
											? 'right'
											: undefined
									}
									w={header.id === 'edit' ? '10' : undefined}
								>
									{flexRender(
										header.column.columnDef.header,
										header.getContext(),
									)}
								</Table.ColumnHeader>
							))}
						</Table.Row>
					))}
				</Table.Header>
				<Table.Body>
					{table.getRowModel().rows.map((row) => (
						<Table.Row
							key={row.original.ip}
							bg='bg.panel'
							borderColor='border'
							_hover={{ bg: 'bg.subtle' }}
							transition='background 0.15s'
						>
							{row.getVisibleCells().map((cell) => (
								<React.Fragment key={cell.id}>
									{flexRender(cell.column.columnDef.cell, cell.getContext())}
								</React.Fragment>
							))}
						</Table.Row>
					))}
				</Table.Body>
			</Table.Root>
		</GridPage>
	);
}
