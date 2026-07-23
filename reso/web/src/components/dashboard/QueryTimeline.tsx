import { Chart, useChart } from '@chakra-ui/charts';
import { Box, Text } from '@chakra-ui/react';
import { useCallback, useMemo } from 'react';
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

const HOUR_MS = 3_600_000;
const DAY_MS = 86_400_000;

const MAX_CHART_POINTS = 300;

// recharts can't render thousands of points smoothly, so cap it.
function decimate(data: TimelineBucket[], maxPoints: number): TimelineBucket[] {
	if (data.length <= maxPoints) {
		return data;
	}

	const chunkSize = Math.ceil(data.length / maxPoints);

	const merged: TimelineBucket[] = [];
	for (let i = 0; i < data.length; i += chunkSize) {
		const chunk = data.slice(i, i + chunkSize);
		merged.push({
			ts: chunk[0].ts,
			total: chunk.reduce((sum, d) => sum + d.total, 0),
			blocked: chunk.reduce((sum, d) => sum + d.blocked, 0),
			cached: chunk.reduce((sum, d) => sum + d.cached, 0),
			errors: chunk.reduce((sum, d) => sum + d.errors, 0),
			sum_duration: chunk.reduce((sum, d) => sum + d.sum_duration, 0),
			bucket_duration: chunk.reduce((sum, d) => sum + d.bucket_duration, 0),
		});
	}
	return merged;
}

// Bucket width varies by age (minute/hour/day), so raw counts aren't comparable, convert to a rate instead.
function normalizeToHourlyRate(data: TimelineBucket[]) {
	return data.map((d) => {
		const factor = HOUR_MS / d.bucket_duration;
		return {
			...d,
			total: Math.round(d.total * factor),
			blocked: Math.round(d.blocked * factor),
			cached: Math.round(d.cached * factor),
			errors: Math.round(d.errors * factor),
		};
	});
}

function formatTime(ts: number, showDate: boolean, showTime: boolean) {
	return new Date(ts).toLocaleString([], {
		month: showDate ? 'short' : undefined,
		day: showDate ? 'numeric' : undefined,
		hour: showTime ? '2-digit' : undefined,
		minute: showTime ? '2-digit' : undefined,
	});
}

function formatTooltipLabel(ts: number, showDate: boolean, showTime: boolean) {
	return new Date(ts).toLocaleString([], {
		year: showDate ? 'numeric' : undefined,
		month: showDate ? 'short' : undefined,
		day: showDate ? 'numeric' : undefined,
		hour: showTime ? '2-digit' : undefined,
		minute: showTime ? '2-digit' : undefined,
	});
}

// Check if a series of time line buckets span over multiple days.
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
	const normalized = useMemo(
		() => normalizeToHourlyRate(decimate(data, MAX_CHART_POINTS)),
		[data],
	);

	const chart = useChart({
		data: normalized,
		series: [
			{ name: 'total', color: 'pink.solid' },
			{ name: 'blocked', color: 'orange.solid' },
			{ name: 'cached', color: 'blue.solid' },
			{ name: 'errors', color: 'red.solid' },
		],
	});

	const showTime = useCallback(
		(ts: number) => {
			const point = (chart.data as typeof normalized).find((d) => d.ts === ts);
			return (point?.bucket_duration ?? 0) < DAY_MS;
		},
		[chart.data],
	);

	const showDate = useMemo(
		() => spansMultipleDays(chart.data as typeof normalized),
		[chart.data],
	);

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
						tickFormatter={(ts) => formatTime(ts, showDate, showTime(ts))}
					/>
					<YAxis axisLine={false} tickLine={false} />
					<Tooltip
						cursor={false}
						labelFormatter={(label) => {
							const ts = Number(label);
							return formatTooltipLabel(ts, showDate, showTime(ts));
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
