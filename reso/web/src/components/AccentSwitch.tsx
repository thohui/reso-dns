import { Switch } from '@chakra-ui/react';
import type { MouseEvent } from 'react';

interface Props {
	checked: boolean;
	onCheckedChange: (checked: boolean) => void;
	onBlur?: () => void;
	onClick?: (e: MouseEvent<HTMLLabelElement>) => void;
	size?: Switch.RootProps['size'];
	'aria-label'?: string;
}

export function AccentSwitch({
	checked,
	onCheckedChange,
	onBlur,
	onClick,
	size,
	'aria-label': ariaLabel,
}: Props) {
	return (
		<Switch.Root
			checked={checked}
			onCheckedChange={(details) => onCheckedChange(details.checked)}
			onBlur={onBlur}
			onClick={onClick}
			size={size}
		>
			<Switch.HiddenInput aria-label={ariaLabel} />
			<Switch.Control
				bg={checked ? 'accent' : 'bg.elevated'}
				boxShadow={`0 0 0 1px {colors.${checked ? 'accent' : 'border.input'}}`}
				_hover={{ boxShadow: '0 0 0 1px {colors.accent}' }}
				transition='all 0.2s ease'
			>
				<Switch.Thumb bg='fg' />
			</Switch.Control>
		</Switch.Root>
	);
}
