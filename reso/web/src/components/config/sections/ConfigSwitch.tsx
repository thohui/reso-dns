import { type Control, Controller, type Path } from 'react-hook-form';
import { AccentSwitch } from '@/components/AccentSwitch';
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
				<AccentSwitch
					checked={field.value as boolean}
					onCheckedChange={field.onChange}
					onBlur={() => field.onBlur()}
				/>
			)}
		/>
	);
}
