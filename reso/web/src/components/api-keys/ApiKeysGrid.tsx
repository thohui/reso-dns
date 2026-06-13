import { ConfirmDeleteButton } from '@/components/ConfirmDeleteButton';
import { GridPage } from '@/components/GridPage';
import type { ApiKey } from '@/lib/api/api-keys';
import { formatTimeAgo } from '@/lib/time';
import { Box, Button, HStack, Icon, Input, Table, Text } from '@chakra-ui/react';
import {
	createColumnHelper,
	flexRender,
	getCoreRowModel,
	useReactTable,
} from '@tanstack/react-table';
import { Key, Plus, Search } from 'lucide-react';
import React, { useMemo } from 'react';

const columnHelper = createColumnHelper<ApiKey>();

interface ApiKeysGridProps {
	keys: ApiKey[];
	onDelete: (id: string) => void;
	onAdd: () => void;
	page: number;
	totalPages: number | null;
	total: number | null;
	hasMore?: boolean;
	isLoading?: boolean;
	onPageChange: (page: number) => void;
	search: string;
	onSearchChange: (value: string) => void;
}

export function ApiKeysGrid({
	keys,
	onDelete,
	onAdd,
	page,
	totalPages,
	total,
	hasMore,
	isLoading,
	onPageChange,
	search,
	onSearchChange,
}: ApiKeysGridProps) {
	const columns = useMemo(
		() => [
			columnHelper.accessor('display_name', {
				header: 'Display Name',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4'>
						<Text fontSize='sm' fontWeight='500'>
							{getValue()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('created_by', {
				header: 'Created by',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4'>
						<Text fontSize='sm' fontWeight='500'>
							{getValue()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('created_at', {
				header: 'Created',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{formatTimeAgo(getValue())}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('expires_at', {
				header: 'Expires',
				cell: ({ getValue }) => {
					const expiresAt = getValue();
					const label = expiresAt != null ? formatTimeAgo(expiresAt) : 'Never';
					return (
						<Table.Cell py='3.5' px='4' textAlign='right'>
							<Text color='fg.muted' fontSize='sm'>{label}</Text>
						</Table.Cell>
					);
				},
			}),
			columnHelper.display({
				id: 'delete',
				header: '',
				cell: ({ row }) => (
					<ConfirmDeleteButton onConfirm={() => onDelete(row.original.id)} />
				),
			}),
		],
		[onDelete],
	);

	const table = useReactTable({
		data: keys,
		columns,
		getCoreRowModel: getCoreRowModel(),
		manualPagination: true,
		pageCount: totalPages ?? undefined,
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
					placeholder='Search keys...'
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
			<Button
				bg='accent'
				color='fg'
				_hover={{ bg: 'accent.hover' }}
				h='8'
				fontSize='sm'
				size='sm'
				onClick={onAdd}
			>
				<Icon as={Plus} boxSize='3.5' mr='1' />
				New Key
			</Button>
		</HStack>
	);

	return (
		<GridPage
			toolbar={toolbar}
			isEmpty={keys.length === 0}
			emptyIcon={search ? Search : Key}
			emptyTitle={search ? 'No keys match your search' : 'No API keys yet'}
			emptySubtitle={search ? 'Try adjusting your search' : 'Click "New Key" to create one'}
			page={page}
			totalPages={totalPages}
			total={total}
			totalLabel='keys'
			hasMore={hasMore}
			isLoading={isLoading}
			onPageChange={onPageChange}
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
										header.id === 'created_at' || header.id === 'expires_at'
											? 'right'
											: undefined
									}
									w={header.id === 'delete' ? '10' : undefined}
								>
									{flexRender(header.column.columnDef.header, header.getContext())}
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
