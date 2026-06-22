import type { MatchType } from '@/lib/api/domain-rules';
import { Badge, Tooltip } from '@chakra-ui/react';

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
					px='2'
					py='0.5'
					fontSize='xs'
					fontWeight='500'
					fontFamily="'Mozilla Text', sans-serif"
					borderRadius='md'
					borderWidth='1px'
					color='fg.muted'
					bg='bg.subtle'
					borderColor='border'
					textTransform='none'
					cursor='default'
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
