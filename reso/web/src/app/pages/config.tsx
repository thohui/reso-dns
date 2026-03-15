import {
	Box,
	Button,
	Field,
	Heading,
	HStack,
	Icon,
	Input,
	Switch,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import {
	FileText,
	Plus,
	RotateCcw,
	Save,
	Server,
	Shield,
	Timer,
	Trash2,
} from 'lucide-react';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import { UpstreamSpecSchema } from '../..//lib/config/schema';
import { ConfigField } from '../../components/config/ConfigField';
import { ConfigSection } from '../../components/config/ConfigSection';
import { DurationInput } from '../../components/config/DurationInput';
import {
	PROTOCOL_COLORS,
	UpstreamPicker,
} from '../../components/config/UpstreamPicker';
import { toastError } from '../../components/Toaster';
import { useConfig, useConfigQueryKey } from '../../hooks/useConfig';
import { useUpdateConfig } from '../../hooks/useUpdateConfig';
import type { ConfigModel, Upstream } from '../../lib/api/config';
import { detectProtocol, getProviderGroup } from '../../lib/config/providers';
import { hexToRgba } from '../../lib/theme';

const schema = z.object({
	upstreams: z.array(UpstreamSpecSchema),
	timeout: z.coerce.number().min(1),
	rate_limit_enabled: z.boolean(),
	rate_limit_window: z.coerce.number().int().min(1),
	rate_limit_max: z.coerce.number().int().min(1),
	logs_enabled: z.boolean(),
	logs_retention_secs: z.coerce.number().int().min(60),
	logs_truncate_interval_secs: z.coerce.number().int().min(60),
	block_icloud_private_relay: z.boolean(),
	block_firefox_canary: z.boolean(),
	block_designated_resolver: z.boolean(),
});

export default function ConfigPage() {
	const config = useConfig();

	const [pickerOpen, setPickerOpen] = useState(false);

	const form = useForm({
		resolver: zodResolver(schema),
		defaultValues: {
			upstreams: config.data.dns.forwarder.upstreams,
			timeout: config.data.dns.timeout,
			rate_limit_enabled: config.data.dns.rate_limit.enabled,
			rate_limit_window: config.data.dns.rate_limit.window_duration,
			rate_limit_max: config.data.dns.rate_limit.max_queries_per_window,
			logs_enabled: config.data.logs.enabled,
			logs_retention_secs: config.data.logs.retention_secs,
			logs_truncate_interval_secs: config.data.logs.truncate_interval_secs,
			block_icloud_private_relay:
				config.data.dns.security.block_icloud_private_relay,
			block_firefox_canary: config.data.dns.security.block_firefox_canary,
			block_designated_resolver:
				config.data.dns.security.block_designated_resolver,
		},
	});

	const updateConfig = useUpdateConfig();

	const queryClient = useQueryClient();

	const handleSave = form.handleSubmit((data) => {
		const updatedConfig: ConfigModel = {
			...config.data,
			dns: {
				...config.data.dns,
				timeout: data.timeout,
				forwarder: {
					...config.data.dns.forwarder,
					upstreams: data.upstreams,
				},
				rate_limit: {
					enabled: data.rate_limit_enabled,
					window_duration: data.rate_limit_window,
					max_queries_per_window: data.rate_limit_max,
				},
				security: {
					block_icloud_private_relay: data.block_icloud_private_relay,
					block_firefox_canary: data.block_firefox_canary,
					block_designated_resolver: data.block_designated_resolver,
				},
			},
			logs: {
				enabled: data.logs_enabled,
				retention_secs: data.logs_retention_secs,
				truncate_interval_secs: data.logs_truncate_interval_secs,
			},
		};

		updateConfig.mutate(updatedConfig, {
			onSuccess: (data) => {
				// Mark the current values as the new base, needed to reset the save and reset buttons.
				form.reset({
					upstreams: data.dns.forwarder.upstreams,
					timeout: data.dns.timeout,
					rate_limit_enabled: data.dns.rate_limit.enabled,
					rate_limit_window: data.dns.rate_limit.window_duration,
					rate_limit_max: data.dns.rate_limit.max_queries_per_window,
					logs_enabled: data.logs.enabled,
					logs_retention_secs: data.logs.retention_secs,
					logs_truncate_interval_secs: data.logs.truncate_interval_secs,
					block_icloud_private_relay:
						data.dns.security.block_icloud_private_relay,
					block_firefox_canary: data.dns.security.block_firefox_canary,
					block_designated_resolver:
						data.dns.security.block_designated_resolver,
				});
				// Update cache
				queryClient.setQueryData(useConfigQueryKey, () => data);
			},
			onError: (e) => toastError(e),
		});
	});

	const handleAddUpstream = (upstream: Upstream) => {
		const currentUpstreams = form.getValues('upstreams');

		form.setValue('upstreams', [...currentUpstreams, upstream], {
			shouldDirty: true,
			shouldValidate: true,
			shouldTouch: true,
		});
	};

	const handleRemoveUpstream = (upstream: Upstream) => {
		const updatedUpStreams = form
			.getValues('upstreams')
			.filter((v) => v !== upstream);

		form.setValue('upstreams', updatedUpStreams, {
			shouldDirty: true,
			shouldValidate: true,
			shouldTouch: true,
		});
	};

	const upstreams = form.watch('upstreams');
	const rateLimitEnabled = form.watch('rate_limit_enabled');
	const logsEnabled = form.watch('logs_enabled');
	const logsRetentionSecs = form.watch('logs_retention_secs') as number;
	const logsTruncateIntervalSecs = form.watch(
		'logs_truncate_interval_secs',
	) as number;

	const blockIcloudPrivateRelayEnabled = form.watch(
		'block_icloud_private_relay',
	);
	const blockFirefoxCanaryEnabled = form.watch('block_firefox_canary');
	const blockDesignatedResolverEnabled = form.watch(
		'block_designated_resolver',
	);

	const hasChanges = form.formState.isDirty;

	return (
		<Box>
			<HStack justify='space-between' mb='8'>
				<Box>
					<Heading size='lg'>Configuration</Heading>
				</Box>
				<HStack gap='3'>
					<Button
						variant='ghost'
						color='fg.muted'
						_hover={{ bg: 'bg.subtle' }}
						onClick={() => {
							form.reset();
						}}
						disabled={!hasChanges}
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
						loading={form.formState.isLoading || updateConfig.isPending}
						disabled={!hasChanges || !form.formState.isValid}
						px='5'
						h='9'
						fontSize='sm'
					>
						<Icon as={Save} boxSize='3.5' mr='2' />
						Save Changes
					</Button>
				</HStack>
			</HStack>

			<ConfigSection
				title='Upstream Servers'
				description='DNS servers that Reso forwards queries to.'
				icon={Server}
			>
				{upstreams.length > 0 ? (
					<Box
						borderRadius='lg'
						borderWidth='1px'
						borderColor='border'
						overflow='hidden'
						mb='4'
					>
						{upstreams.map((upstream, i) => {
							const maybeProviderGroup = getProviderGroup(upstream);

							const providerSlug = maybeProviderGroup?.slug ?? 'C';
							const providerName = maybeProviderGroup?.name ?? 'Custom';
							const providerBadgeColor = maybeProviderGroup?.color ?? '#E91E78';

							const protocol = detectProtocol(upstream);

							const protocolColor = PROTOCOL_COLORS[protocol] ?? '#71717a';

							return (
								<HStack
									key={upstream}
									justify='space-between'
									py='3'
									px='4'
									borderBottomWidth={i < upstreams.length - 1 ? '1px' : '0'}
									borderColor='border'
									transition='background 0.1s ease'
									_hover={{ bg: 'bg.subtle' }}
								>
									<HStack gap='3'>
										<Box
											w='8'
											h='8'
											borderRadius='md'
											bg={hexToRgba(providerBadgeColor, 0.1)}
											borderWidth='1px'
											borderColor={hexToRgba(providerBadgeColor, 0.3)}
											display='flex'
											alignItems='center'
											justifyContent='center'
											fontWeight='bold'
											fontSize='2xs'
											color={providerBadgeColor}
											flexShrink={0}
										>
											{providerSlug}
										</Box>
										<Box>
											<HStack gap='2'>
												<Text fontSize='sm' fontWeight='500'>
													{providerName}
												</Text>
												<Box
													px='1.5'
													py='0.5'
													borderRadius='md'
													bg={hexToRgba(protocolColor, 0.09)}
													borderWidth='1px'
													borderColor={hexToRgba(protocolColor, 0.19)}
												>
													<Text
														fontSize='2xs'
														fontWeight='600'
														color={protocolColor}
														letterSpacing='0.02em'
													>
														{protocol}
													</Text>
												</Box>
											</HStack>
											<Text
												fontSize='xs'
												color='fg.muted'
												fontFamily='mono'
												mt='0.5'
											>
												{upstream}
											</Text>
										</Box>
									</HStack>
									<Button
										size='sm'
										variant='ghost'
										color='fg.faint'
										_hover={{ color: 'status.error', bg: 'transparent' }}
										onClick={() => handleRemoveUpstream(upstream)}
										disabled={upstreams.length <= 1} // Disallow all upstreams from being deleted
										p='1'
										h='auto'
										minW='auto'
									>
										<Icon as={Trash2} boxSize='4' />
									</Button>
								</HStack>
							);
						})}
					</Box>
				) : (
					<Box
						textAlign='center'
						py='10'
						mb='4'
						borderWidth='1px'
						borderColor='border'
						borderRadius='lg'
						borderStyle='dashed'
					>
						<Icon as={Server} boxSize='8' color='fg.faint' mb='3' />
						<Text color='fg.muted' fontSize='sm'>
							No upstream servers configured
						</Text>
						<Text color='fg.faint' fontSize='xs' mt='1'>
							Add at least one upstream server
						</Text>
					</Box>
				)}

				<Button
					variant='ghost'
					borderWidth='1px'
					borderColor='border'
					borderStyle='dashed'
					color='fg.muted'
					_hover={{
						bg: 'bg.subtle',
						borderColor: 'accent',
						color: 'accent.fg',
					}}
					onClick={() => setPickerOpen(true)}
					w='full'
					py='5'
					borderRadius='lg'
					transition='all 0.15s ease'
				>
					<Icon as={Plus} boxSize='4' mr='2' />
					Add Upstream Server
				</Button>
			</ConfigSection>

			<ConfigSection title='Timeout' icon={Timer}>
				<ConfigField
					label='Timeout'
					description='Maximum upstream response wait time per query (ms).'
				>
					<Field.Root invalid={!!form.formState.errors.timeout}>
						<Input
							type='number'
							{...form.register('timeout', { valueAsNumber: true })}
						/>
						{form.formState.errors.timeout?.message && (
							<Field.ErrorText color='status.error'>
								{form.formState.errors.timeout.message}
							</Field.ErrorText>
						)}
					</Field.Root>
				</ConfigField>
			</ConfigSection>

			<ConfigSection
				title='Security'
				description='Options to block certain types of queries for improved privacy and security.'
				icon={Shield}
			>
				<ConfigField
					label='iCloud Private Relay Bypass'
					description='Prevents Apple devices from bypassing Reso by routing DNS queries through iCloud Private Relay.'
					align='center'
				>
					<Switch.Root
						checked={blockIcloudPrivateRelayEnabled}
						onCheckedChange={({ checked }) =>
							form.setValue('block_icloud_private_relay', checked, {
								shouldDirty: true,
								shouldTouch: true,
								shouldValidate: true,
							})
						}
					>
						<Switch.HiddenInput />
						<Switch.Control
							bg={blockIcloudPrivateRelayEnabled ? 'accent' : 'bg.elevated'}
							borderWidth='1px'
							borderColor={
								blockIcloudPrivateRelayEnabled ? 'accent' : 'border.input'
							}
							_hover={{ borderColor: 'accent' }}
							transition='all 0.2s ease'
						>
							<Switch.Thumb bg='fg' />
						</Switch.Control>
					</Switch.Root>
				</ConfigField>
				<ConfigField
					label='Firefox DoH Bypass'
					description="Blocks Firefox's built-in DNS-over-HTTPS detection, which would otherwise bypass Reso."
					align='center'
				>
					<Switch.Root
						checked={blockFirefoxCanaryEnabled}
						onCheckedChange={({ checked }) =>
							form.setValue('block_firefox_canary', checked, {
								shouldDirty: true,
								shouldTouch: true,
								shouldValidate: true,
							})
						}
					>
						<Switch.HiddenInput />
						<Switch.Control
							bg={blockFirefoxCanaryEnabled ? 'accent' : 'bg.elevated'}
							borderWidth='1px'
							borderColor={
								blockFirefoxCanaryEnabled ? 'accent' : 'border.input'
							}
							_hover={{ borderColor: 'accent' }}
							transition='all 0.2s ease'
						>
							<Switch.Thumb bg='fg' />
						</Switch.Control>
					</Switch.Root>
				</ConfigField>
				<ConfigField
					label='Auto Resolver Discovery'
					description='Prevents devices from auto-discovering alternative DNS resolvers via the resolver.arpa zone, keeping all DNS traffic routed through Reso.'
					align='center'
				>
					<Switch.Root
						checked={blockDesignatedResolverEnabled}
						onCheckedChange={({ checked }) =>
							form.setValue('block_designated_resolver', checked, {
								shouldDirty: true,
								shouldTouch: true,
								shouldValidate: true,
							})
						}
					>
						<Switch.HiddenInput />
						<Switch.Control
							bg={blockDesignatedResolverEnabled ? 'accent' : 'bg.elevated'}
							borderWidth='1px'
							borderColor={
								blockDesignatedResolverEnabled ? 'accent' : 'border.input'
							}
							_hover={{ borderColor: 'accent' }}
							transition='all 0.2s ease'
						>
							<Switch.Thumb bg='fg' />
						</Switch.Control>
					</Switch.Root>
				</ConfigField>
			</ConfigSection>

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
					<Switch.Root
						checked={rateLimitEnabled}
						onCheckedChange={({ checked }) =>
							form.setValue('rate_limit_enabled', checked, {
								shouldDirty: true,
								shouldTouch: true,
								shouldValidate: true,
							})
						}
					>
						<Switch.HiddenInput />
						<Switch.Control
							bg={rateLimitEnabled ? 'accent' : 'bg.elevated'}
							borderWidth='1px'
							borderColor={rateLimitEnabled ? 'accent' : 'border.input'}
							_hover={{ borderColor: 'accent' }}
							transition='all 0.2s ease'
						>
							<Switch.Thumb bg='fg' />
						</Switch.Control>
					</Switch.Root>
				</ConfigField>
				<ConfigField
					label='Window Duration'
					description='Length of each rate limit window (seconds).'
				>
					<Field.Root invalid={!!form.formState.errors.rate_limit_window}>
						<Input
							type='number'
							min={1}
							step={1}
							{...form.register('rate_limit_window', { valueAsNumber: true })}
						/>
						{form.formState.errors.rate_limit_window?.message && (
							<Field.ErrorText color='status.error'>
								{form.formState.errors.rate_limit_window.message}
							</Field.ErrorText>
						)}
					</Field.Root>
				</ConfigField>
				<ConfigField
					label='Max Queries'
					description='Maximum queries allowed per client per window.'
				>
					<Field.Root invalid={!!form.formState.errors.rate_limit_max}>
						<Input
							type='number'
							min={1}
							step={1}
							{...form.register('rate_limit_max', { valueAsNumber: true })}
						/>
						{form.formState.errors.rate_limit_max?.message && (
							<Field.ErrorText color='status.error'>
								{form.formState.errors.rate_limit_max.message}
							</Field.ErrorText>
						)}
					</Field.Root>
				</ConfigField>
			</ConfigSection>

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
					<Switch.Root
						checked={logsEnabled}
						onCheckedChange={({ checked }) =>
							form.setValue('logs_enabled', checked, {
								shouldDirty: true,
								shouldTouch: true,
								shouldValidate: true,
							})
						}
					>
						<Switch.HiddenInput />
						<Switch.Control
							bg={logsEnabled ? 'accent' : 'bg.elevated'}
							borderWidth='1px'
							borderColor={logsEnabled ? 'accent' : 'border.input'}
							_hover={{ borderColor: 'accent' }}
							transition='all 0.2s ease'
						>
							<Switch.Thumb bg='fg' />
						</Switch.Control>
					</Switch.Root>
				</ConfigField>
				<ConfigField
					label='Retention'
					description='How long to keep activity logs before cleanup.'
				>
					<DurationInput
						value={logsRetentionSecs}
						onChange={(v) =>
							form.setValue('logs_retention_secs', v, {
								shouldDirty: true,
								shouldValidate: true,
							})
						}
						min={60}
					/>
				</ConfigField>
				<ConfigField
					label='Cleanup Interval'
					description='How often to run log cleanup.'
				>
					<DurationInput
						value={logsTruncateIntervalSecs}
						onChange={(v) =>
							form.setValue('logs_truncate_interval_secs', v, {
								shouldDirty: true,
								shouldValidate: true,
							})
						}
						min={60}
					/>
				</ConfigField>
			</ConfigSection>

			{pickerOpen && (
				<UpstreamPicker
					onClose={() => setPickerOpen(false)}
					onAdd={handleAddUpstream}
					existingUpstreams={upstreams}
				/>
			)}
		</Box>
	);
}
