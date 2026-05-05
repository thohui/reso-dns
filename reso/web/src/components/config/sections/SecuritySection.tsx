import { ConfigField } from '@/components/config/ConfigField';
import { ConfigSection } from '@/components/config/ConfigSection';
import { Shield } from 'lucide-react';
import { type Control } from 'react-hook-form';
import { ConfigSwitch } from './ConfigSwitch';
import type { FormValues } from '@/lib/config/schema';

export function SecuritySection({ control }: { control: Control<FormValues> }) {
	return (
		<ConfigSection
			title='Security'
			description='Options to block certain types of queries for improved privacy and security.'
			icon={Shield}
		>
			<ConfigField
				label='Block iCloud Private Relay'
				description='Prevents Apple devices from bypassing Reso by routing DNS queries through iCloud Private Relay.'
				align='center'
			>
				<ConfigSwitch
					control={control}
					name='security.block_icloud_private_relay'
				/>
			</ConfigField>
			<ConfigField
				label='Block Firefox Canary'
				description="Blocks Firefox's built-in DNS-over-HTTPS detection, which would otherwise bypass Reso."
				align='center'
			>
				<ConfigSwitch control={control} name='security.block_firefox_canary' />
			</ConfigField>
			<ConfigField
				label='Block Auto Resolver Discovery'
				description='Prevents devices from auto-discovering alternative DNS resolvers via the resolver.arpa zone, keeping all DNS traffic routed through Reso.'
				align='center'
			>
				<ConfigSwitch
					control={control}
					name='security.block_designated_resolver'
				/>
			</ConfigField>
		</ConfigSection>
	);
}
