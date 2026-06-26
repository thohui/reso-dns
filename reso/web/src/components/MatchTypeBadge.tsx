import { Badge, Tooltip } from '@chakra-ui/react';
import type { MatchType } from '@/lib/api/domain-rules';

const config: Record<MatchType, { label: string; tooltip: string }> = {
	domain: {
		label: 'Domain',
		tooltip: 'Matches the domain and all its subdomains',
	},
	wildcard: {
		label: 'Wildcard',
		tooltip: 'Matches subdomains only, not the domain itself',
	},
	exact: {
		label: 'Exact',
		tooltip: 'Matches only this exact domain, no subdomains',
	},
};

export function MatchTypeBadge({ matchType }: { matchType: MatchType }) {
	const { label, tooltip } = config[matchType];
	return (
		<Tooltip.Root openDelay={300}>
			<Tooltip.Trigger asChild>
				<Badge
					px='2.5'
					py='1'
					borderRadius='md'
					size='md'
					fontSize='xs'
					fontWeight='500'
					fontFamily="'Mozilla Text', sans-serif"
					borderWidth='1px'
					color='accent.fg'
					bg='accent.muted'
					borderColor='accent.subtle'
					variant='subtle'
				>
					{label}
				</Badge>
			</Tooltip.Trigger>
			<Tooltip.Positioner>
				<Tooltip.Content fontSize='xs'>{tooltip}</Tooltip.Content>
			</Tooltip.Positioner>
		</Tooltip.Root>
	);
}
