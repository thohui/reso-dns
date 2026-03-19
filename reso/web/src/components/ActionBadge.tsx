import { Badge } from '@chakra-ui/react';
import type { ListAction } from '../lib/api/domain-rules';

interface Props {
	action: ListAction;
}

export function ActionBadge({ action }: Props) {
	return (
		<Badge
			size='sm'
			colorPalette={action === 'block' ? 'red' : 'green'}
			variant='subtle'
			textTransform='capitalize'
		>
			{action}
		</Badge>
	);
}
