import type { StatusInfo } from '@/lib/status-info';
import { Badge } from '@chakra-ui/react';

interface Props {
	statusInfo: StatusInfo;
	size: 'xs' | 'sm' | 'md' | 'lg';
}

export function StatusBadge({ statusInfo, size }: Props) {
	const { label, color, bg } = statusInfo;
	return (
		<Badge
			px='2.5'
			py='1'
			borderRadius='md'
			fontSize='xs'
			fontWeight='600'
			textTransform='uppercase'
			letterSpacing='0.03em'
			bg={bg}
			color={color}
			size={size}
		>
			{label}
		</Badge>
	);
}
