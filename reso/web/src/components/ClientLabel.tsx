import { Text } from '@chakra-ui/react';

interface Props {
	ip: string | null;
	name?: string | null;
}

export function ClientLabel({ ip, name }: Props) {
	if (!name) return <>{ip ?? 'unknown'}</>;
	return (
		<>
			{name}{' '}
			<Text as='span' fontFamily='data' fontSize='xs' color='fg.subtle'>
				({ip})
			</Text>
		</>
	);
}
