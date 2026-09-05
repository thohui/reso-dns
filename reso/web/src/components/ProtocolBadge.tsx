import { Badge } from '@chakra-ui/react';
import type { DnsProtocol } from '@/lib/api/activity';

interface Props {
	protocol: DnsProtocol | null;
	size: 'xs' | 'sm' | 'md' | 'lg';
}

export function ProtocolBadge({ protocol, size }: Props) {
	const label = protocol ?? 'Unknown';
	return (
		<Badge
			bg='accent.muted'
			color='accent.fg'
			size={size}
			variant='subtle'
			fontFamily="'Mozilla Text', sans-serif"
			fontSize='xs'
			fontWeight='500'
			textTransform='capitalize'
		>
			{label}
		</Badge>
	);
}
