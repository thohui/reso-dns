import { Switch } from '@chakra-ui/react';
import { type Control, Controller, type Path } from 'react-hook-form';
import type { FormValues } from '@/lib/config/schema';

export function ConfigSwitch({
	control,
	name,
}: {
	control: Control<FormValues>;
	name: Path<FormValues>;
}) {
	return (
		<Controller
			control={control}
			name={name}
			render={({ field }) => (
				<Switch.Root
					checked={field.value as boolean}
					onCheckedChange={({ checked }) => field.onChange(checked)}
					onBlur={() => field.onBlur()}
				>
					<Switch.HiddenInput />
					<Switch.Control
						bg={field.value ? 'accent' : 'bg.elevated'}
						boxShadow={`0 0 0 1px {colors.${field.value ? 'accent' : 'border.input'}}`}
						_hover={{ boxShadow: '0 0 0 1px {colors.accent}' }}
						transition='all 0.2s ease'
					>
						<Switch.Thumb bg='fg' />
					</Switch.Control>
				</Switch.Root>
			)}
		/>
	);
}
