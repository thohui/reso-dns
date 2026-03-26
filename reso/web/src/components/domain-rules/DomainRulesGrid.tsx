import { ActionBadge } from '@/components/ActionBadge';
import { ConfirmDeleteButton } from '@/components/ConfirmDeleteButton';
import { GridPage } from '@/components/GridPage';
import { ToggleButton } from '@/components/ToggleButton';
import type { DomainRule } from '@/lib/api/domain-rules';
import { formatTimeAgo } from '@/lib/time';
import { Box, Icon, IconButton, Input, Table, Text } from '@chakra-ui/react';
import {
	createColumnHelper,
	flexRender,
	getCoreRowModel,
	useReactTable,
} from '@tanstack/react-table';
import { Globe, Pencil, Search } from 'lucide-react';
import React, { useMemo } from 'react';

const columnHelper = createColumnHelper<DomainRule>();

interface DomainRulesGridProps {
	rules: DomainRule[];
	page: number;
	totalPages: number | null;
	total: number | null;
	onPageChange: (page: number) => void;
	search: string;
	onSearchChange: (value: string) => void;
	onRemove: (domain: string) => void;
	onToggle: (domain: string) => void;
	onEdit: (rule: DomainRule) => void;
	isLoading: boolean;
}

export function DomainRulesGrid({
	rules,
	page,
	totalPages,
	total,
	onPageChange,
	search,
	onSearchChange,
	onRemove,
	onToggle,
	onEdit,
	isLoading,
}: DomainRulesGridProps) {
	const columns = useMemo(
		() => [
			columnHelper.accessor('domain', {
				header: 'Domain',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4'>
						<Text
							fontFamily="'Mozilla Text', sans-serif"
							fontSize='sm'
							fontWeight='500'
							whiteSpace='nowrap'
							overflow='hidden'
							textOverflow='ellipsis'
						>
							{getValue()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('action', {
				header: 'Action',
				cell: ({ getValue }) => (
					<Table.Cell py='3.5' px='4'>
						<ActionBadge action={getValue()} />
					</Table.Cell>
				),
			}),
			columnHelper.accessor('created_at', {
				header: 'Added',
				cell: (info) => (
					<Table.Cell py='3.5' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{formatTimeAgo(info.getValue())}
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
							label='rule'
							onToggle={() => onToggle(row.original.domain)}
						/>
					</Table.Cell>
				),
			}),
			columnHelper.display({
				id: 'edit',
				header: '',
				cell: ({ row }) => (
					<Table.Cell py='3.5' px='4'>
						<IconButton
							variant='ghost'
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
			columnHelper.display({
				id: 'delete',
				header: '',
				cell: ({ row }) => (
					<ConfirmDeleteButton
						onConfirm={() => onRemove(row.original.domain)}
					/>
				),
			}),
		],
		[onRemove, onToggle, onEdit],
	);

	const table = useReactTable({
		data: rules,
		columns,
		manualPagination: true,
		pageCount: totalPages ?? undefined,
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
				placeholder='Search domains...'
				value={search}
				onChange={(e) => onSearchChange(e.target.value)}
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
			isLoading={isLoading}
			isEmpty={rules.length === 0}
			emptyIcon={search ? Search : Globe}
			emptyTitle={search ? 'No domains match your search' : 'No rules yet'}
			emptySubtitle={
				search ? 'Try adjusting your search' : 'Click "Add Rule" to get started'
			}
			page={page}
			totalPages={totalPages}
			total={total}
			totalLabel='rules'
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
										header.id === 'created_at'
											? 'right'
											: header.id === 'status'
												? 'center'
												: undefined
									}
									w={
										header.id === 'edit' || header.id === 'delete'
											? '10'
											: undefined
									}
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
							key={row.original.domain}
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
