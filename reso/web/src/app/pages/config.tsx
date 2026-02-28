import {
	Box,
	Button,
	Field,
	Heading,
	HStack,
	Icon,
	Input,
	Text,
} from '@chakra-ui/react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { Plus, RotateCcw, Save, Server, Timer, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import z from 'zod';
import { useUpdateConfig } from '../..//hooks/useUpdateConfig';
import { UpstreamSpecSchema } from '../..//lib/config/schema';
import { ConfigField } from '../../components/config/ConfigField';
import { ConfigSection } from '../../components/config/ConfigSection';
import {
	PROTOCOL_COLORS,
	UpstreamPicker,
} from '../../components/config/UpstreamPicker';
import { toastError } from '../../components/Toaster';
import { useConfig, useConfigQueryKey } from '../../hooks/useConfig';
import type { ConfigModel, Upstream } from '../../lib/api/config';
import { detectProtocol, getProviderGroup } from '../../lib/config/providers';
import { hexToRgba } from '../../lib/theme';

const schema = z.object({
	upstreams: z.array(UpstreamSpecSchema),
	timeout: z.coerce.number().min(1),
});

export default function ConfigPage() {

	const config = useConfig();

	const [pickerOpen, setPickerOpen] = useState(false);

	const form = useForm({
		resolver: zodResolver(schema),
		defaultValues: {
			upstreams: config.data.dns.forwarder.upstreams,
			timeout: config.data.dns.timeout,
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
			},
		};

		updateConfig.mutate(updatedConfig, {
			onSuccess: (data) => {
				// Mark the current values as the new base, needed to reset the save and reset buttons.
				form.reset({
					upstreams: data.dns.forwarder.upstreams,
					timeout: data.dns.timeout,
				});
				// Update cache
				queryClient.setQueryData(useConfigQueryKey, () => data);
			},
			onError: (e) => toastError(e)
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
													bg={`${PROTOCOL_COLORS[protocol] || '#71717a'}18`}
													borderWidth='1px'
													borderColor={`${PROTOCOL_COLORS[protocol] || '#71717a'}30`}
												>
													<Text
														fontSize='2xs'
														fontWeight='600'
														color={PROTOCOL_COLORS[protocol] || '#71717a'}
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
						<Input type='number' {...form.register('timeout')} />
						{form.formState.errors.timeout?.message && (
							<Field.ErrorText color='status.error'>
								{form.formState.errors.timeout.message}
							</Field.ErrorText>
						)}
					</Field.Root>
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
