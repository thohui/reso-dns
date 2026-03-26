import { Box, Icon, Input, Table, Text } from '@chakra-ui/react';
import {
	createColumnHelper,
	flexRender,
	getCoreRowModel,
	useReactTable,
} from '@tanstack/react-table';
import { Globe, Search } from 'lucide-react';
import React, { useMemo, useState } from 'react';
import type { LocalRecord } from '@/lib/api/local-records';
import { formatTimeAgo } from '@/lib/time';
import { ConfirmDeleteButton } from '@/components/ConfirmDeleteButton';
import { GridPage } from '@/components/GridPage';
import { RecordTypeBadge } from '@/components/RecordTypeBadge';
import { ToggleButton } from '@/components/ToggleButton';

const TYPE_COLORS: Record<number, string> = {
	1: '#60a5fa', // A
	28: '#a78bfa', // AAAA
	5: '#34d399', // CNAME
};

const columnHelper = createColumnHelper<LocalRecord>();

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

	// TODO: move this to the server.
	const filtered = useMemo(
		() =>
			records.filter(
				(r) =>
					!search ||
					r.name.toLowerCase().includes(search.toLowerCase()) ||
					r.value.toLowerCase().includes(search.toLowerCase()),
			),
		[records, search],
	);

	const columns = useMemo(
		() => [
			columnHelper.accessor('name', {
				header: 'Name',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4'>
						<Text fontFamily='mono' fontSize='sm' fontWeight='500'>
							{getValue()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('record_type', {
				header: 'Type',
				cell: ({ getValue }) => {
					return (
						<Table.Cell py='3.5' px='4'>
							<RecordTypeBadge recordType={getValue()} size='md' />
						</Table.Cell>
					);
				},
			}),
			columnHelper.accessor('value', {
				header: 'Value',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4'>
						<Text fontFamily='mono' fontSize='sm' color='fg.muted'>
							{getValue()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('ttl', {
				header: 'TTL',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{getValue()}s
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('created_at', {
				header: 'Added',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{formatTimeAgo(getValue())}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.display({
				id: 'status',
				header: 'Status',
				cell: ({ row }) => (
					<Table.Cell py='3.5' px='4' textAlign='center'>
						<ToggleButton
							enabled={row.original.enabled}
							label='record'
							onToggle={() => onToggle(row.original.id)}
						/>
					</Table.Cell>
				),
			}),
			columnHelper.display({
				id: 'delete',
				header: '',
				cell: ({ row }) => (
					<ConfirmDeleteButton onConfirm={() => onRemove(row.original.id)} />
				),
			}),
		],
		[onRemove, onToggle],
	);

	const table = useReactTable({
		data: filtered,
		columns,
		getCoreRowModel: getCoreRowModel(),
	});

	const toolbar = (
		<Box position='relative'>
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
	);

	return (
		<GridPage
			toolbar={toolbar}
			isEmpty={filtered.length === 0}
			emptyIcon={search ? Search : Globe}
			emptyTitle={
				search ? 'No records match your search' : 'No local records yet'
			}
			emptySubtitle={
				search
					? 'Try adjusting your search'
					: 'Click "Add Record" to get started'
			}
		>
			<Table.Root size='sm'>
				<Table.Header>
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
										header.id === 'ttl' || header.id === 'created_at'
											? 'right'
											: header.id === 'status'
												? 'center'
												: undefined
									}
									w={header.id === 'delete' ? '10' : undefined}
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
							key={row.original.id}
							bg='bg.panel'
							borderColor='border'
							_hover={{ bg: 'bg.subtle' }}
							transition='background 0.15s'
							opacity={row.original.enabled ? 1 : 0.5}
						>
							{row.getVisibleCells().map((cell) => {
								return (
									<React.Fragment key={cell.id}>
										{flexRender(cell.column.columnDef.cell, cell.getContext())}
									</React.Fragment>
								);
							})}
						</Table.Row>
					))}
				</Table.Body>
			</Table.Root>
		</GridPage>
	);
}
