import { Chart, useChart } from '@chakra-ui/charts';
import { Box, Text } from '@chakra-ui/react';
import { useMemo } from 'react';
import {
	Area,
	AreaChart,
	CartesianGrid,
	Tooltip,
	XAxis,
	YAxis,
} from 'recharts';
import type { TimelineBucket } from '@/lib/api/stats';

interface Props {
	data: TimelineBucket[];
	loading?: boolean;
}

const DAY_MS = 86_400_000;

function formatTs(
	ts: number,
	showDate: boolean,
	showTime: boolean,
	showYear: boolean,
) {
	return new Date(ts).toLocaleString([], {
		year: showYear ? 'numeric' : undefined,
		month: showDate ? 'short' : undefined,
		day: showDate ? 'numeric' : undefined,
		hour: showTime ? '2-digit' : undefined,
		minute: showTime ? '2-digit' : undefined,
	});
}

function spansMultipleDays(data: TimelineBucket[]): boolean {
	if (data.length === 0) {
		return false;
	}
	const first = new Date(data[0].ts);
	const last = new Date(data[data.length - 1].ts);
	return (
		first.getFullYear() !== last.getFullYear() ||
		first.getMonth() !== last.getMonth() ||
		first.getDate() !== last.getDate()
	);
}

export function QueryTimeline({ data, loading }: Props) {
	const chart = useChart({
		data,
		series: [
			{ name: 'total', color: 'pink.solid' },
			{ name: 'blocked', color: 'orange.solid' },
			{ name: 'cached', color: 'blue.solid' },
			{ name: 'errors', color: 'red.solid' },
		],
	});

	// same width for every bucket, so show the time on all of them or none
	const showTime = (data[0]?.bucket_duration ?? 0) < DAY_MS;
	const showDate = useMemo(() => spansMultipleDays(data), [data]);

	if (loading || data.length === 0) {
		return (
			<Box
				bg='bg.panel'
				borderRadius='xl'
				borderWidth='1px'
				borderColor='border'
				p='5'
				h='full'
				display='flex'
				flexDir='column'
			>
				<Text
					color='fg.subtle'
					fontSize='xs'
					fontWeight='500'
					textTransform='uppercase'
					letterSpacing='0.05em'
					mb='4'
					flexShrink={0}
				>
					Query Timeline
				</Text>
				<Box
					flex='1'
					minH='0'
					display='flex'
					alignItems='center'
					justifyContent='center'
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
			h='full'
			display='flex'
			flexDir='column'
		>
			<Text
				color='fg.subtle'
				fontSize='xs'
				fontWeight='500'
				textTransform='uppercase'
				letterSpacing='0.05em'
				mb='4'
				flexShrink={0}
			>
				Query Timeline
			</Text>

			<Chart.Root flex='1' minH='0' chart={chart}>
				<AreaChart data={chart.data} responsive>
					<CartesianGrid
						stroke={chart.color('border.muted')}
						vertical={false}
					/>
					<XAxis
						axisLine={false}
						tickLine={false}
						dataKey={chart.key('ts')}
						tickFormatter={(ts) => formatTs(ts, showDate, showTime, false)}
					/>
					<YAxis axisLine={false} tickLine={false} />
					<Tooltip
						cursor={false}
						labelFormatter={(label) => {
							const ts = Number(label);
							return formatTs(ts, showDate, showTime, showDate);
						}}
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
