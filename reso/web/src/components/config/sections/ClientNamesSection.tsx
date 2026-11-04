import {
	Button,
	Field,
	HStack,
	Icon,
	Input,
	Stack,
	Text,
} from '@chakra-ui/react';
import { Plus, Trash2 } from 'lucide-react';
import { type Control, useController, useWatch } from 'react-hook-form';
import { ConfigField } from '@/components/config/ConfigField';
import { ConfigSection } from '@/components/config/ConfigSection';
import type { FormValues } from '@/lib/config/schema';
import { ConfigSwitch } from './ConfigSwitch';

export function ClientNamesSection({
	control,
}: {
	control: Control<FormValues>;
}) {
	const { field, formState } = useController({
		control,
		name: 'ptr_servers',
	});
	const auto = useWatch({ control, name: 'ptr_servers_auto' });
	const servers = field.value;
	const errors = formState.errors.ptr_servers;

	const update = (i: number, value: string) =>
		field.onChange(servers.map((s, idx) => (idx === i ? value : s)));
	const remove = (i: number) =>
		field.onChange(servers.filter((_, idx) => idx !== i));

	return (
		<ConfigSection
			title='Client Names'
			description='Servers Reso asks for client hostnames with reverse (PTR) lookups, usually your router or DHCP server.'
		>
			<ConfigField
				label='Use Default Gateway'
				description='Ask the router this machine routes through, it is detected again when it changes.'
				align='center'
			>
				<ConfigSwitch control={control} name='ptr_servers_auto' />
			</ConfigField>
			{!auto && (
				<>
					<Stack gap='2' mb='3'>
						{servers.length === 0 && (
							<Text color='fg.muted' fontSize='sm'>
								No servers configured, clients will not be named.
							</Text>
						)}
						{servers.map((server, i) => {
							const error = errors?.[i]?.message;
							return (
								// biome-ignore lint/suspicious/noArrayIndexKey: entries are plain strings that can repeat while typing
								<Field.Root key={i} invalid={!!error}>
									<HStack w='full'>
										<Input
											aria-label={`PTR server ${i + 1}`}
											value={server}
											placeholder='192.168.1.1'
											fontFamily='mono'
											fontSize='sm'
											onChange={(e) => update(i, e.target.value)}
											borderColor='border.input'
											_hover={{ borderColor: 'accent.subtle' }}
											_focus={{ borderColor: 'accent.subtle' }}
										/>
										<Button
											aria-label='Remove server'
											size='sm'
											variant='ghost'
											color='fg.faint'
											_hover={{ color: 'status.error', bg: 'transparent' }}
											onClick={() => remove(i)}
											p='1'
											h='auto'
											minW='auto'
										>
											<Icon as={Trash2} boxSize='4' />
										</Button>
									</HStack>
									{error && (
										<Field.ErrorText color='status.error'>
											{error}
										</Field.ErrorText>
									)}
								</Field.Root>
							);
						})}
					</Stack>
					<Button
						variant='ghost'
						borderWidth='1px'
						borderColor='border'
						borderRadius='lg'
						color='fg.muted'
						_hover={{
							bg: 'bg.subtle',
							borderColor: 'accent.subtle',
							color: 'fg',
						}}
						onClick={() => field.onChange([...servers, ''])}
						w='full'
						py='5'
						transition='all 0.15s ease'
					>
						<Icon as={Plus} boxSize='4' mr='2' />
						Add Server
					</Button>
				</>
			)}
		</ConfigSection>
	);
}
