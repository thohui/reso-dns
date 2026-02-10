import { useEffect, useMemo, useRef, useState } from 'react';

export type UptimeValue = {
	ready: boolean;
	text: string; // '3d 04:05:06' or '04:05:06'
	days: number;
	hours: number;
	minutes: number;
	seconds: number;
	totalSeconds: number;
};

function pad2(n: number): string {
	return n < 10 ? `0${n}` : String(n);
}

export function useUptime(startedAtMs?: number | null): UptimeValue {
	const startMs = useMemo<number | null>(() => {
		if (startedAtMs == null) return null;
		if (!Number.isFinite(startedAtMs)) return null;
		return startedAtMs;
	}, [startedAtMs]);

	// Store epoch seconds so state only changes once per second
	const [nowSec, setNowSec] = useState<number>(() =>
		Math.floor(Date.now() / 1000),
	);

	const timerRef = useRef<number | null>(null);

	useEffect(() => {
		const clear = () => {
			if (timerRef.current !== null) {
				window.clearTimeout(timerRef.current);
				timerRef.current = null;
			}
		};

		// Don’t tick until we have a start time
		if (startMs === null) {
			clear();
			return;
		}

		const tick = () => {
			setNowSec((prev) => {
				const next = Math.floor(Date.now() / 1000);
				return next === prev ? prev : next;
			});

			// Align to next second boundary for stable display
			const delay = 1000 - (Date.now() % 1000);
			timerRef.current = window.setTimeout(tick, delay);
		};

		timerRef.current = window.setTimeout(tick, 1000 - (Date.now() % 1000));
		return clear;
	}, [startMs]);

	return useMemo((): UptimeValue => {
		if (startMs === null) {
			return {
				ready: false,
				text: '--:--:--',
				days: 0,
				hours: 0,
				minutes: 0,
				seconds: 0,
				totalSeconds: 0,
			};
		}

		const nowMs = nowSec * 1000;

		let totalSeconds = Math.floor((nowMs - startMs) / 1000);
		if (totalSeconds < 0) totalSeconds = 0; // clamp (future start)

		const days = Math.floor(totalSeconds / 86400);
		const hours = Math.floor((totalSeconds % 86400) / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = totalSeconds % 60;

		const core = `${pad2(hours)}:${pad2(minutes)}:${pad2(seconds)}`;
		const text = days > 0 ? `${days}d ${core}` : core;

		return { ready: true, text, days, hours, minutes, seconds, totalSeconds };
	}, [startMs, nowSec]);
}
