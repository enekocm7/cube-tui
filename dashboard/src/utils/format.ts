import type { Time } from "../types/types";
import { Modifier } from "../types/types";

export function formatMillis(ms: number): string {
	const total = Math.floor(ms);
	const minutes = Math.floor(total / 60000);
	const seconds = Math.floor((total % 60000) / 1000);
	const millis = total % 1000;
	return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${String(millis).padStart(3, "0")}`;
}

export function effectiveMs(time: Time): number | null {
	switch (time.modifier) {
		case Modifier.DNF:
			return null;
		case Modifier.PlusTwo:
			return time.timestamp_in_millis + 2000;
		default:
			return time.timestamp_in_millis;
	}
}

export function formatTime(time: Time): string {
	const ms = effectiveMs(time);
	if (ms === null) return `DNF(${formatMillis(time.timestamp_in_millis)})`;
	if (time.modifier === Modifier.PlusTwo) return `${formatMillis(ms)}+`;
	return formatMillis(ms);
}

function trimmedMean(effectives: (number | null)[], n: number): number | "DNF" {
	const dnfCount = effectives.filter((ms) => ms === null).length;
	if (dnfCount > 1) return "DNF";

	const values = effectives.map((ms) => ms ?? Infinity);
	const sorted = [...values].sort((a, b) => a - b);
	const trimmed = sorted.slice(1, n - 1); // drop best and worst
	return Math.round(trimmed.reduce((a, b) => a + b, 0) / trimmed.length);
}

/** Average of last `n` times (WCA rules: drop best + worst, 1 DNF allowed as worst). */
export function computeAo(times: Time[], n: number): number | "DNF" | null {
	if (times.length < n) return null;
	const last = times.slice(-n);
	return trimmedMean(last.map(effectiveMs), n);
}

/** Best rolling average of `n` times across the whole session. */
export function computeBestAo(times: Time[], n: number): number | "DNF" | null {
	if (times.length < n) return null;
	const effectives = times.map(effectiveMs);

	const window: (number | null)[] = Array.from({ length: n }, () => null);
	let best: number | null = null;
	for (let i = 0; i <= effectives.length - n; i++) {
		for (let j = 0; j < n; j++) {
			window[j] = effectives[i + j];
		}
		const avg = trimmedMean(window, n);
		if (avg === "DNF") continue;
		if (best === null || avg < best) {
			best = avg;
		}
	}
	return best;
}

export function computeBest(times: Time[]): number | null {
	const effective = times
		.map(effectiveMs)
		.filter((ms): ms is number => ms !== null);
	if (effective.length === 0) return null;
	return Math.min(...effective);
}

export function computeMean(times: Time[]): number | null {
	const effective = times
		.map(effectiveMs)
		.filter((ms): ms is number => ms !== null);
	if (effective.length === 0) return null;
	return Math.round(effective.reduce((a, b) => a + b, 0) / effective.length);
}

export function computeWorst(times: Time[]): number | null {
	const effective = times
		.map(effectiveMs)
		.filter((ms): ms is number => ms !== null);
	if (effective.length === 0) return null;
	return Math.max(...effective);
}

export function computeStdDev(times: Time[]): number | null {
	const effective = times
		.map(effectiveMs)
		.filter((ms): ms is number => ms !== null);
	if (effective.length === 0) return null;
	const mean = effective.reduce((a, b) => a + b, 0) / effective.length;
	const variance =
		effective.reduce((sum, ms) => sum + Math.pow(ms - mean, 2), 0) /
		effective.length;
	return Math.round(Math.sqrt(variance));
}

export function computeCountByModifier(
	times: Time[],
	modifier: Modifier,
): number {
	return times.filter((t) => t.modifier === modifier).length;
}

export function computeSuccessRate(times: Time[]): number | null {
	if (times.length === 0) return null;
	const dnfs = computeCountByModifier(times, Modifier.DNF);
	return Math.round(((times.length - dnfs) / times.length) * 100);
}

export function computeSessionDuration(times: Time[]): number | null {
	if (times.length < 2) return null;
	const timestamps = times.map((t) => t.solved_at_unix_ms).filter(Boolean);
	if (timestamps.length < 2) return null;
	return Math.max(...timestamps) - Math.min(...timestamps);
}

export interface DistributionBucket {
	start: number;
	end: number;
	count: number;
}

export function computePercentile(times: Time[]): number | null {
	if (times.length === 0) return null;
	const last = times[times.length - 1];
	const lastMs = effectiveMs(last);
	if (lastMs === null) return null;
	const valid = times
		.map(effectiveMs)
		.filter((ms): ms is number => ms !== null);
	const slower = valid.filter((ms) => ms > lastMs).length;
	return Math.round((slower / valid.length) * 100);
}

export function computeDistribution(
	times: Time[],
	bucketCount = 12,
): DistributionBucket[] {
	const effective = times
		.map(effectiveMs)
		.filter((ms): ms is number => ms !== null);
	if (effective.length === 0) return [];

	const min = Math.min(...effective);
	const max = Math.max(...effective);
	const range = max - min || 1;
	const bucketSize = range / bucketCount;

	const counts: number[] = Array.from({ length: bucketCount }, () => 0);
	for (const ms of effective) {
		const idx = Math.min(Math.floor((ms - min) / bucketSize), bucketCount - 1);
		counts[idx]++;
	}

	const buckets: DistributionBucket[] = [];
	for (let i = 0; i < bucketCount; i++) {
		const start = min + i * bucketSize;
		const end = i === bucketCount - 1 ? max + 1 : min + (i + 1) * bucketSize;
		buckets.push({ start, end, count: counts[i] });
	}

	return buckets;
}

export function formatDuration(ms: number): string {
	const totalMinutes = Math.floor(ms / 60000);
	const hours = Math.floor(totalMinutes / 60);
	const minutes = totalMinutes % 60;
	if (hours > 0) {
		return `${hours}h ${minutes}m`;
	}
	return `${minutes}m`;
}

export function formatDate(unix_ms: number): string {
	if (!unix_ms) return "—";
	return new Date(unix_ms).toLocaleDateString("en", {
		month: "short",
		day: "numeric",
	});
}

export function formatStat(ms: number | "DNF" | null): string {
	if (ms === null) return "—";
	if (ms === "DNF") return "DNF";
	return formatMillis(ms);
}
