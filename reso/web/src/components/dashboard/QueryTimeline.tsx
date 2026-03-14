import { Chart, useChart } from '@chakra-ui/charts';
import { Box, Text } from '@chakra-ui/react';
import {
	Area,
	AreaChart,
	CartesianGrid,
	Tooltip,
	XAxis,
	YAxis,
} from 'recharts';
import type { TimelineBucket } from '../../lib/api/stats';

interface Props {
	data: TimelineBucket[];
	loading?: boolean;
}

function formatTime(ts: number) {
	return new Date(ts).toLocaleTimeString([], {
		hour: '2-digit',
		minute: '2-digit',
	});
}

export function QueryTimeline({ data, loading }: Props) {
	const chart = useChart({
		data,
		series: [
			{ name: 'total', color: 'pink.solid' },
			{ name: 'blocked', color: 'orange.solid' },
			{ name: 'cached', color: 'blue.solid' },
		],
	});

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
					mb='4'
				>
					Query Timeline
				</Text>
				<Box
					display='flex'
					alignItems='center'
					justifyContent='center'
					h='250px'
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
				mb='4'
			>
				Query Timeline
			</Text>

			<Chart.Root maxH='sm' chart={chart}>
				<AreaChart data={chart.data} responsive>
					<CartesianGrid
						stroke={chart.color('border.muted')}
						vertical={false}
					/>
					<XAxis
						axisLine={false}
						tickLine={false}
						dataKey={chart.key('ts')}
						tickFormatter={formatTime}
					/>
					<YAxis axisLine={false} tickLine={false} />
					<Tooltip
						cursor={false}
						labelFormatter={(label) => new Date(Number(label)).toLocaleString()}
						animationDuration={100}
						content={<Chart.Tooltip />}
					/>
					{chart.series.map((item) => (
						<Area
							type='monotone'
							key={item.name}
							isAnimationActive={false}
							dataKey={chart.key(item.name)}
							fill={chart.color(item.color)}
							fillOpacity={0.2}
							stroke={chart.color(item.color)}
						/>
					))}
				</AreaChart>
			</Chart.Root>
		</Box>
	);
}
