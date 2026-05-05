import { ConfigField } from '@/components/config/ConfigField';
import { ConfigSection } from '@/components/config/ConfigSection';
import { DurationInput } from '@/components/config/DurationInput';
import { Timer } from 'lucide-react';
import { Controller, type Control } from 'react-hook-form';
import type { FormValues } from '@/lib/config/schema';

export function TimeoutSection({ control }: { control: Control<FormValues> }) {
	return (
		<ConfigSection title='Timeout' icon={Timer}>
			<ConfigField
				label='Timeout'
				description='Maximum upstream response wait time per query'
			>
				<Controller
					control={control}
					name='timeout'
					render={({ field, fieldState }) => (
						<DurationInput
							allowedUnits={['seconds', 'milliseconds']}
							conversion='milliseconds'
							value={field.value}
							min={1}
							onChange={field.onChange}
							invalid={!!fieldState.error}
							errorText={fieldState.error?.message}
						/>
					)}
				/>
			</ConfigField>
		</ConfigSection>
	);
}
