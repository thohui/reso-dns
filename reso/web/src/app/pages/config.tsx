import { LogRetentionSection } from '@/components/config/sections/LogRetentionSection';
import { RateLimitSection } from '@/components/config/sections/RateLimitSection';
import { SecuritySection } from '@/components/config/sections/SecuritySection';
import { TimeoutSection } from '@/components/config/sections/TimeoutSection';
import { UpstreamsSection } from '@/components/config/sections/UpstreamsSection';
import { toastError } from '@/components/Toaster';
import { useConfig, useConfigQueryKey } from '@/hooks/config/useConfig';
import { useUpdateConfig } from '@/hooks/config/useUpdateConfig';
import type { ConfigModel } from '@/lib/api/config';
import { configSchema } from '@/lib/config/schema';
import { Box, Button, HStack, Icon } from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { RotateCcw, Save } from 'lucide-react';
import { useForm } from 'react-hook-form';

export default function ConfigPage() {
	const config = useConfig();
	const queryClient = useQueryClient();
	const updateConfig = useUpdateConfig();

	const form = useForm({
		resolver: zodResolver(configSchema),
		defaultValues: {
			upstreams: config.data.dns.forwarder.upstreams,
			timeout: config.data.dns.timeout,
			rate_limit: config.data.dns.rate_limit,
			security: config.data.dns.security,
			logs: config.data.logs,
		},
	});

	const handleSave = form.handleSubmit((data) => {
		const updatedConfig: ConfigModel = {
			...config.data,
			dns: {
				...config.data.dns,
				timeout: data.timeout,
				forwarder: { ...config.data.dns.forwarder, upstreams: data.upstreams },
				rate_limit: data.rate_limit,
				security: data.security,
			},
			logs: data.logs,
		};

		updateConfig.mutate(updatedConfig, {
			onSuccess: (updated) => {
				form.reset({
					upstreams: updated.dns.forwarder.upstreams,
					timeout: updated.dns.timeout,
					rate_limit: updated.dns.rate_limit,
					security: updated.dns.security,
					logs: updated.logs,
				});
				queryClient.setQueryData(useConfigQueryKey, () => updated);
			},
			onError: toastError,
		});
	});

	const { control, formState } = form;

	return (
		<Box>
			<HStack justify='flex-end' mb='4' gap='3'>
				<Button
					variant='ghost'
					color='fg.muted'
					_hover={{ bg: 'bg.subtle' }}
					onClick={() => form.reset()}
					disabled={!formState.isDirty}
					px='4'
					h='9'
					fontSize='sm'
				>
					<Icon as={RotateCcw} boxSize='3.5' mr='2' />
					Reset
				</Button>
				<Button
					bg='accent'
					color='fg'
					_hover={{ bg: 'accent.hover' }}
					onClick={handleSave}
					loading={formState.isLoading || updateConfig.isPending}
					disabled={!formState.isDirty || !formState.isValid}
					px='5'
					h='9'
					fontSize='sm'
				>
					<Icon as={Save} boxSize='3.5' mr='2' />
					Save Changes
				</Button>
			</HStack>

			<UpstreamsSection control={control} />
			<TimeoutSection control={control} />
			<SecuritySection control={control} />
			<RateLimitSection control={control} />
			<LogRetentionSection control={control} />
		</Box>
	);
}
