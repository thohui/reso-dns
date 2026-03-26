import { recordTypeName } from '@/lib/dns';
import { Badge } from '@chakra-ui/react';

interface Props {
	recordType: string | number;
	size: 'xs' | 'sm' | 'md' | 'lg';
}
export function RecordTypeBadge({ recordType, size }: Props) {
	const recordTypeString =
		typeof recordType === 'string' ? recordType : recordTypeName(recordType);

	return (
		<Badge
			px='2.5'
			py='1'
			borderRadius='md'
			size={size}
			bg='accent.muted'
			color='accent.fg'
			variant='subtle'
			fontFamily="'Mozilla Text', sans-serif"
			fontSize='xs'
			fontWeight='500'
		>
			{recordTypeString}
		</Badge>
	);
}
