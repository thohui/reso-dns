import { Chart, useChart } from '@chakra-ui/charts';
import { Box, Text } from '@chakra-ui/react';
import { Pie, PieChart, Sector, Tooltip } from 'recharts';
import type { TopEntry } from '../../lib/api/stats';

const COLORS = [
	'pink.solid',
	'blue.solid',
	'green.solid',
	'orange.solid',
	'yellow.solid',
	'purple.solid',
	'red.solid',
	'teal.solid',
	'cyan.solid',
	'gray.solid',
];

interface Props {
	title: string;
	data: TopEntry[];
	loading?: boolean;
}

export function TopDonutChart({ title, data, loading }: Props) {
	const chartData = data.map((entry, i) => ({
		...entry,
		color: COLORS[i % COLORS.length],
	}));

	const chart = useChart({ data: chartData });

	if (loading || data.length === 0) {
		return (
			<Box
				bg='bg.panel'
				borderRadius='xl'
				borderWidth='1px'
				borderColor='border'
				p='5'
			>
				<Text
					color='fg.subtle'
					fontSize='xs'
					fontWeight='500'
					textTransform='uppercase'
					letterSpacing='0.05em'
					mb='3'
				>
					{title}
				</Text>
				<Box
					display='flex'
					alignItems='center'
					justifyContent='center'
					h='200px'
				>
					<Text color='fg.faint' fontSize='sm'>
						{loading ? 'Loading...' : 'No data available'}
					</Text>
				</Box>
			</Box>
		);
	}

	return (
		<Box
			bg='bg.panel'
			borderRadius='xl'
			borderWidth='1px'
			borderColor='border'
			p='5'
		>
			<Text
				color='fg.subtle'
				fontSize='xs'
				fontWeight='500'
				textTransform='uppercase'
				letterSpacing='0.05em'
				mb='3'
			>
				{title}
			</Text>

			<Box display='flex' alignItems='center' gap='4'>
				<Chart.Root chart={chart} boxSize='140px'>
					<PieChart responsive>
						<Tooltip
							cursor={false}
							animationDuration={100}
							content={<Chart.Tooltip hideLabel />}
						/>
						<Pie
							innerRadius={40}
							outerRadius={65}
							isAnimationActive={false}
							data={chart.data}
							dataKey={chart.key('count')}
							nameKey='name'
							shape={(props) => (
								<Sector {...props} fill={chart.color(props.payload.color)} />
							)}
						/>
					</PieChart>
				</Chart.Root>
				<Box flex='1' overflow='hidden'>
					{chartData.slice(0, 5).map((entry) => (
						<Box
							key={entry.name}
							display='flex'
							alignItems='center'
							gap='2'
							py='1'
						>
							<Box
								w='2'
								h='2'
								borderRadius='full'
								bg={chart.color(entry.color)}
								flexShrink={0}
							/>
							<Text fontSize='xs' color='fg.muted' truncate flex='1'>
								{entry.name}
							</Text>
							<Text fontSize='xs' color='fg' fontWeight='500' flexShrink={0}>
								{entry.count}
							</Text>
						</Box>
					))}
				</Box>
			</Box>
		</Box>
	);
}
