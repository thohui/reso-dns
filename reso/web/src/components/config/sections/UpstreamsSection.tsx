import { Box, Button, HStack, Icon, Text } from '@chakra-ui/react';
import { Plus, Server, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { type Control, useController } from 'react-hook-form';
import { ConfigSection } from '@/components/config/ConfigSection';
import {
	PROTOCOL_COLORS,
	UpstreamPicker,
} from '@/components/config/UpstreamPicker';
import { detectProtocol, getProviderGroup } from '@/lib/config/providers';
import type { FormValues } from '@/lib/config/schema';
import { hexToRgba } from '@/lib/theme';

export function UpstreamsSection({
	control,
}: {
	control: Control<FormValues>;
}) {
	const { field } = useController({ control, name: 'upstreams' });
	const upstreams: string[] = field.value;
	const [pickerOpen, setPickerOpen] = useState(false);

	const append = (spec: string) => field.onChange([...upstreams, spec]);
	const remove = (i: number) =>
		field.onChange(upstreams.filter((_, idx) => idx !== i));

	return (
		<ConfigSection
			title='Upstream Servers'
			description='DNS servers that Reso forwards queries to. Queries are distributed across servers in round-robin order.'
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
						const providerName = maybeProviderGroup?.name ?? 'Custom';
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
											wordBreak='break-all'
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
									onClick={() => remove(i)}
									disabled={upstreams.length <= 1}
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
				borderRadius='lg'
				color='fg.muted'
				_hover={{ bg: 'bg.subtle', borderColor: 'accent.subtle', color: 'fg' }}
				onClick={() => setPickerOpen(true)}
				w='full'
				py='5'
				transition='all 0.15s ease'
			>
				<Icon as={Plus} boxSize='4' mr='2' />
				Add Upstream Server
			</Button>
			{pickerOpen && (
				<UpstreamPicker
					onClose={() => setPickerOpen(false)}
					onAdd={append}
					existingUpstreams={upstreams}
				/>
			)}
		</ConfigSection>
	);
}
