import { Box, Button, HStack, Icon, Text } from '@chakra-ui/react';
import { ChevronLeft, ChevronRight, type LucideIcon } from 'lucide-react';

interface GridPageProps {
	children: React.ReactNode;
	toolbar?: React.ReactNode;
	isLoading?: boolean;
	isEmpty?: boolean;
	emptyIcon?: LucideIcon;
	emptyTitle?: string;
	emptySubtitle?: string;
	page?: number;
	totalPages?: number | null;
	total?: number | null;
	totalLabel?: string;
	hasMore?: boolean;
	onPageChange?: (page: number) => void;
}

export function GridPage({
	children,
	toolbar,
	isLoading,
	isEmpty,
	emptyIcon: EmptyIcon,
	emptyTitle,
	emptySubtitle,
	page,
	totalPages,
	total,
	totalLabel,
	hasMore,
	onPageChange,
}: GridPageProps) {
	const hasPagination =
		!!onPageChange && (totalPages == null || totalPages > 1);

	const totalString = total != null ? total.toLocaleString() : '';

	const handlePageChange = (newPage: number) => {
		// this is a bit hacky
		document
			.getElementById('main-scroll')
			?.scrollTo({ top: 0, behavior: 'auto' });
		onPageChange?.(newPage);
	};

	return (
		<Box>
			{toolbar && <Box mb='4'>{toolbar}</Box>}

			<Box
				bg='bg.panel'
				borderRadius='lg'
				borderWidth='1px'
				borderColor='border'
				overflow='hidden'
				opacity={isLoading ? 0.6 : 1}
				transition='opacity 0.15s'
			>
				{isEmpty && !isLoading ? (
					<Box py='14' textAlign='center'>
						{EmptyIcon && (
							<Icon
								as={EmptyIcon}
								boxSize='10'
								color='fg.subtle'
								mb='3'
								display='block'
								mx='auto'
							/>
						)}
						{emptyTitle && (
							<Text color='fg.muted' fontSize='sm' mb='1'>
								{emptyTitle}
							</Text>
						)}
						{emptySubtitle && (
							<Text color='fg.subtle' fontSize='xs'>
								{emptySubtitle}
							</Text>
						)}
					</Box>
				) : (
					<Box overflowX='auto'>{children}</Box>
				)}
			</Box>

			{hasPagination && (
				<HStack justify='space-between' mt='4' px='1'>
					<Text fontSize='xs' color='fg.muted'>
						{totalString}
						{totalLabel ? ` ${totalLabel}` : ''}
					</Text>
					<HStack gap='2'>
						<Button
							size='xs'
							variant='ghost'
							color='fg.muted'
							_hover={{ bg: 'bg.subtle' }}
							disabled={(page ?? 0) === 0}
							onClick={() => handlePageChange((page ?? 0) - 1)}
						>
							<Icon as={ChevronLeft} boxSize='3.5' />
							Prev
						</Button>
						<Text fontSize='xs' color='fg.muted'>
							{totalPages != null
								? `${(page ?? 0) + 1} / ${totalPages}`
								: `${(page ?? 0) + 1}`}
						</Text>
						<Button
							size='xs'
							variant='ghost'
							color='fg.muted'
							_hover={{ bg: 'bg.subtle' }}
							disabled={
								totalPages != null ? (page ?? 0) >= totalPages - 1 : !hasMore
							}
							onClick={() => {
								handlePageChange((page ?? 0) + 1);
							}}
						>
							Next
							<Icon as={ChevronRight} boxSize='3.5' />
						</Button>
					</HStack>
				</HStack>
			)}
		</Box>
	);
}
