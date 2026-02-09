import {
	Box,
	Button,
	HStack,
	Icon,
	Input,
	Text,
	VStack,
} from '@chakra-ui/react';
import { Ban, Globe, Search, Trash2 } from 'lucide-react';
import { useState } from 'react';
import type { BlockedDomain } from '../../lib/api/blocklist';

interface BlocklistGridProps {
	blocklist: BlockedDomain[];
	onRemove: (domain: string) => void;
}

export function BlocklistGrid({ blocklist, onRemove }: BlocklistGridProps) {
	const [search, setSearch] = useState('');

	const filteredBlocklist = search
		? blocklist.filter((d) =>
				d.domain.toLowerCase().includes(search.toLowerCase()),
			)
		: blocklist;

	return (
		<>
			<Box position="relative" mb="4">
				<Box
					position="absolute"
					left="3"
					top="50%"
					transform="translateY(-50%)"
					zIndex="1"
				>
					<Icon as={Search} boxSize="4" color="fg.subtle" />
				</Box>
				<Input
					placeholder="Search blocked domains..."
					value={search}
					onChange={(e) => setSearch(e.target.value)}
					bg="bg.panel"
					borderColor="border.input"
					pl="10"
					_placeholder={{ color: 'fg.subtle' }}
					_focus={{ borderColor: 'accent.subtle' }}
				/>
			</Box>

			<Box
				bg="bg.panel"
				borderRadius="lg"
				borderWidth="1px"
				borderColor="border"
				overflow="hidden"
			>
				<VStack gap="0" align="stretch">
					{filteredBlocklist.length === 0 && (
						<Box py="10" textAlign="center">
							<Icon
								as={search ? Search : Globe}
								boxSize="8"
								color="fg.subtle"
								mb="3"
							/>
							<Text color="fg.muted" fontSize="sm">
								{search
									? 'No domains match your search'
									: 'No domains blocked yet'}
							</Text>
						</Box>
					)}
					{filteredBlocklist.map((domain) => (
						<HStack
							key={domain.domain}
							justify="space-between"
							px="6"
							py="3.5"
							borderBottomWidth="1px"
							borderColor="border"
							_last={{ borderBottom: 'none' }}
							_hover={{ bg: 'bg.subtle' }}
							transition="background 0.15s"
						>
							<HStack gap="3">
								<Icon as={Ban} boxSize="4" color="status.error" />
								<Text fontFamily="mono" fontSize="sm">
									{domain.domain}
								</Text>
							</HStack>
							<Button
								size="sm"
								variant="ghost"
								color="fg.muted"
								_hover={{ color: 'status.error', bg: 'bg.subtle' }}
								onClick={() => onRemove(domain.domain)}
							>
								<Icon as={Trash2} boxSize="4" />
							</Button>
						</HStack>
					))}
				</VStack>
			</Box>
		</>
	);
}
