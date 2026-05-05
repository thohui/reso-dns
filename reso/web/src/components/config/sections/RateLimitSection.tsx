import { ConfigField } from '@/components/config/ConfigField';
import { ConfigSection } from '@/components/config/ConfigSection';
import { DurationInput } from '@/components/config/DurationInput';
import { Field, Input } from '@chakra-ui/react';
import { Shield } from 'lucide-react';
import { Controller, type Control } from 'react-hook-form';
import { ConfigSwitch } from './ConfigSwitch';
import type { FormValues } from '@/lib/config/schema';

export function RateLimitSection({
	control,
}: {
	control: Control<FormValues>;
}) {
	return (
		<ConfigSection
			title='Rate Limiting'
			description='Limit the number of queries per client within a time window.'
			icon={Shield}
		>
			<ConfigField
				label='Enabled'
				description='Enable rate limiting for DNS queries.'
				align='center'
			>
				<ConfigSwitch control={control} name='rate_limit.enabled' />
			</ConfigField>
			<ConfigField
				label='Window Duration'
				description='Length of each rate limit window.'
			>
				<Controller
					control={control}
					name='rate_limit.window_duration'
					render={({ field, fieldState }) => (
						<DurationInput
							allowedUnits={['seconds', 'minutes']}
							conversion='seconds'
							value={field.value}
							onChange={field.onChange}
							min={1}
							invalid={!!fieldState.error}
							errorText={fieldState.error?.message}
						/>
					)}
				/>
			</ConfigField>
			<ConfigField
				label='Max Queries'
				description='Maximum queries allowed per client per window.'
			>
				<Controller
					control={control}
					name='rate_limit.max_queries_per_window'
					render={({ field, fieldState }) => (
						<Field.Root invalid={!!fieldState.error}>
							<Input
								type='number'
								min={1}
								step={1}
								value={field.value}
								onChange={(e) => field.onChange(e.target.valueAsNumber)}
							/>
							{fieldState.error?.message && (
								<Field.ErrorText color='status.error'>
									{fieldState.error.message}
								</Field.ErrorText>
							)}
						</Field.Root>
					)}
				/>
			</ConfigField>
		</ConfigSection>
	);
}
