import { Box, Table, Text, HStack, Icon } from '@chakra-ui/react';
import {
	createColumnHelper,
	flexRender,
	getCoreRowModel,
	useReactTable,
} from '@tanstack/react-table';
import { Globe, List } from 'lucide-react';
import React, { useMemo } from 'react';
import { ConfirmDeleteButton } from '../ConfirmDeleteButton';
import { ToggleButton } from '../ToggleButton';
import { GridPage } from '../GridPage';
import type { ListSubscription } from '../../lib/api/list-subscriptions';
import { formatTimeAgo } from '../../lib/time';
import { ActionBadge } from '../ActionBadge';

const columnHelper = createColumnHelper<ListSubscription>();

interface SubscriptionsGridProps {
	subscriptions: ListSubscription[];
	onRemove: (id: string) => void;
	onToggle: (id: string) => void;
	onToggleSync: (id: string) => void;
}

export function SubscriptionsGrid({
	subscriptions,
	onRemove,
	onToggle,
	onToggleSync,
}: SubscriptionsGridProps) {
	const columns = useMemo(
		() => [
			columnHelper.display({
				id: 'name',
				header: 'Name',
				cell: ({ row }) => {
					const sub = row.original;
					return (
						<Table.Cell py='3.5' px='4'>
							<HStack gap='3'>
								<Icon as={List} boxSize='3.5' color='fg.subtle' />
								<Box>
									<HStack gap='2'>
										<Text fontSize='sm' fontWeight='500'>
											{sub.name}
										</Text>
										<ActionBadge action={sub.list_type} />
									</HStack>
									<Text fontSize='xs' color='fg.subtle' maxW='300px' truncate>
										{sub.url}
									</Text>
								</Box>
							</HStack>
						</Table.Cell>
					);
				},
			}),
			columnHelper.accessor('domain_count', {
				header: 'Domains',
				cell: (info) => (
					<Table.Cell py='3.5' px='4' textAlign='right'>
						<Text color='fg.muted' fontSize='sm'>
							{info.getValue().toLocaleString()}
						</Text>
					</Table.Cell>
				),
			}),
			columnHelper.accessor('last_synced_at', {
				header: 'Last Synced',
				cell: (info) => (
					<Table.Cell py='3.5' px='4' style={{ textAlign: 'right' }}>
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
							label='subscription'
							onToggle={() => onToggle(row.original.id)}
						/>
					</Table.Cell>
				),
			}),
			columnHelper.display({
				id: 'sync_status',
				header: 'Sync',
				cell: ({ row }) => {
					return (
						<Table.Cell py='3.5' px='4' textAlign='center'>
							<ToggleButton
								enabled={row.original.sync_enabled}
								label='subscription'
								onToggle={() => onToggleSync(row.original.id)}
							/>
						</Table.Cell>
					);
				},
			}),
			columnHelper.display({
				id: 'delete',
				header: '',
				cell: ({ row }) => (
					<ConfirmDeleteButton onConfirm={() => onRemove(row.original.id)} />
				),
			}),
		],
		[onRemove, onToggle, onToggleSync],
	);

	const table = useReactTable({
		data: subscriptions,
		columns,
		getCoreRowModel: getCoreRowModel(),
	});

	return (
		<GridPage
			isEmpty={subscriptions.length === 0}
			emptyIcon={Globe}
			emptyTitle='No subscriptions yet'
			emptySubtitle='Click "Add Subscription" to subscribe to a domain list'
		>
			<Table.Root size='sm'>
				<Table.Header>
					{table.getHeaderGroups().map((headerGroup) => (
						<Table.Row
							key={headerGroup.id}
							bg='bg.subtle'
							borderColor='border'>
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
										header.id === 'domain_count' ||
											header.id === 'last_synced_at'
											? 'right'
											: header.id === 'status' || header.id === 'sync_status'
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
