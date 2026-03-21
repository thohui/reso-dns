import { Icon, IconButton } from '@chakra-ui/react';
import { ToggleLeft, ToggleRight } from 'lucide-react';

interface Props {
	enabled: boolean;
	label: string;
	onToggle: () => void;
}

export function ToggleButton({ enabled, label, onToggle }: Props) {
	return (
		<IconButton
			aria-label={enabled ? `Disable ${label}` : `Enable ${label}`}
			variant='plain'
			size='xs'
			color={enabled ? 'status.success' : 'fg.subtle'}
			_hover={{ opacity: 0.8, bg: 'transparent' }}
			transition='all 0.15s'
			onClick={onToggle}
		>
			<Icon as={enabled ? ToggleRight : ToggleLeft} boxSize='5' />
		</IconButton>
	);
}
