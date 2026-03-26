import { getTransportLabel } from '@/lib/api/activity';
import { Badge } from '@chakra-ui/react';

interface Props {
	protocol: string | number;
	size: 'xs' | 'sm' | 'md' | 'lg';
}

export function ProtocolBadge({ protocol, size }: Props) {
	const protocolString =
		typeof protocol === 'string' ? protocol : getTransportLabel(protocol);

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
			{protocolString}
		</Badge>
	);
}
