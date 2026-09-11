import type { Time } from "../types/types";
import { effectiveMs } from "./format";

const MAX_CHART_POINTS = 100;

export function computeTimeTrend(times: Time[]) {
	const chartData: { solve: number; ms: number | null }[] = [];
	const step = Math.max(1, Math.ceil(times.length / MAX_CHART_POINTS));
	let dnfCount = 0;
	let minMs: number | null = null;
	let maxMs: number | null = null;

	for (let index = 0; index < times.length; index++) {
		const ms = effectiveMs(times[index]);
		if (ms === null) {
			dnfCount++;
		} else {
			minMs = minMs === null ? ms : Math.min(minMs, ms);
			maxMs = maxMs === null ? ms : Math.max(maxMs, ms);
		}

		// Only allocate displayed points, including the most recent solve.
		if (index % step === 0 || index === times.length - 1) {
			chartData.push({ solve: index + 1, ms });
		}
	}

	if (minMs === null || maxMs === null) {
		return { chartData, dnfCount, minMs, maxMs, yMin: 0, yMax: 0 };
	}

	const rangePadding = Math.max(1000, Math.round((maxMs - minMs) * 0.08));
	return {
		chartData,
		dnfCount,
		minMs,
		maxMs,
		yMin: Math.max(0, minMs - rangePadding),
		yMax: maxMs + rangePadding,
	};
}
