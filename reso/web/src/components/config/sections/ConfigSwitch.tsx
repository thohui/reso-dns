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
						borderWidth='1px'
						borderColor={field.value ? 'accent' : 'border.input'}
						_hover={{ borderColor: 'accent' }}
						transition='all 0.2s ease'
					>
						<Switch.Thumb bg='fg' />
					</Switch.Control>
				</Switch.Root>
			)}
		/>
	);
}
