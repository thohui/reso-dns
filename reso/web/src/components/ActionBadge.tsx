import type { ListAction } from '@/lib/api/domain-rules';
import { Badge, Icon, chakra } from '@chakra-ui/react';
import { Ban, ShieldCheck } from 'lucide-react';

interface Props {
	action: ListAction;
	onClick?: () => void;
	selected?: boolean;
}

const sharedStyles = {
	px: '2.5',
	py: '1',
	size: 'sm',
	fontFamily: "'Mozilla Text', sans-serif",
	fontSize: 'xs',
	fontWeight: '500',
	textTransform: 'capitalize' as const,
	display: 'inline-flex',
	alignItems: 'center',
	gap: '1',
	borderRadius: 'md',
	borderWidth: '1px',
	transition: 'all 0.15s',
} as const;

function ActionBadgeContent({ action }: { action: ListAction }) {
	const isBlock = action === 'block';
	return (
		<>
			<Icon as={isBlock ? Ban : ShieldCheck} boxSize='3' />
			{action}
		</>
	);
}

export function ActionBadge({ action, onClick, selected }: Props) {
	const isBlock = action === 'block';

	const isClickable = onClick !== undefined;
	const isActive = !isClickable || selected;

	const colorStyles = {
		bg: isActive
			? isBlock
				? 'status.errorMuted'
				: 'status.successMuted'
			: 'bg.subtle',
		color: isActive
			? isBlock
				? 'status.error'
				: 'status.success'
			: 'fg.muted',
		borderColor: isActive
			? isBlock
				? 'status.error'
				: 'status.success'
			: 'border.input',
	};

	if (isClickable) {
		return (
			<chakra.button
				type='button'
				onClick={onClick}
				{...sharedStyles}
				{...colorStyles}
				_hover={{
					bg: isBlock ? 'status.errorMuted' : 'status.successMuted',
					color: isBlock ? 'status.error' : 'status.success',
					borderColor: isBlock ? 'status.error' : 'status.success',
				}}
			>
				<ActionBadgeContent action={action} />
			</chakra.button>
		);
	}

	return (
		<Badge {...sharedStyles} {...colorStyles} variant='subtle'>
			<ActionBadgeContent action={action} />
		</Badge>
	);
}
