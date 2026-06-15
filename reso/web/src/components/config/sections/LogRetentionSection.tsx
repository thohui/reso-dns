import { FileText } from 'lucide-react';
import { type Control, Controller } from 'react-hook-form';
import { ConfigField } from '@/components/config/ConfigField';
import { ConfigSection } from '@/components/config/ConfigSection';
import { DurationInput } from '@/components/config/DurationInput';
import type { FormValues } from '@/lib/config/schema';
import { ConfigSwitch } from './ConfigSwitch';

export function LogRetentionSection({
	control,
}: {
	control: Control<FormValues>;
}) {
	return (
		<ConfigSection
			title='Log Retention'
			description='Configure activity log retention and cleanup.'
			icon={FileText}
		>
			<ConfigField
				label='Enabled'
				description='Automatically clean up old activity logs.'
				align='center'
			>
				<ConfigSwitch control={control} name='logs.enabled' />
			</ConfigField>
			<ConfigField
				label='Retention'
				description='How long to keep activity logs before cleanup.'
			>
				<Controller
					control={control}
					name='logs.retention_secs'
					render={({ field, fieldState }) => (
						<DurationInput
							allowedUnits={['seconds', 'minutes', 'hours', 'days']}
							conversion='seconds'
							value={field.value}
							onChange={field.onChange}
							min={60}
							invalid={!!fieldState.error}
							errorText={fieldState.error?.message}
						/>
					)}
				/>
			</ConfigField>
			<ConfigField
				label='Cleanup Interval'
				description='How often to run log cleanup.'
			>
				<Controller
					control={control}
					name='logs.truncate_interval_secs'
					render={({ field, fieldState }) => (
						<DurationInput
							allowedUnits={['minutes', 'hours', 'days']}
							conversion='seconds'
							value={field.value}
							onChange={field.onChange}
							min={60}
							invalid={!!fieldState.error}
							errorText={fieldState.error?.message}
						/>
					)}
				/>
			</ConfigField>
		</ConfigSection>
	);
}
