import { describe, expect, test } from "bun:test";
import { Modifier, type Time, WcaEvent } from "../src/types/types";
import { computeTimeTrend } from "../src/utils/timeTrend";

function time(ms: number, modifier: Modifier = Modifier.None): Time {
	return {
		timestamp_in_millis: ms,
		modifier,
		event: WcaEvent.Cube3x3,
		scramble: "",
		solved_at_unix_ms: 0,
	};
}

describe("time trend", () => {
	test("preserves penalties, missing points, and padded bounds", () => {
		expect(
			computeTimeTrend([
				time(15000),
				time(10000, Modifier.DNF),
				time(12000, Modifier.PlusTwo),
			]),
		).toEqual({
			chartData: [
				{ solve: 1, ms: 15000 },
				{ solve: 2, ms: null },
				{ solve: 3, ms: 14000 },
			],
			dnfCount: 1,
			minMs: 14000,
			maxMs: 15000,
			yMin: 13000,
			yMax: 16000,
		});
	});

	test("handles empty and all-DNF sessions", () => {
		for (const times of [[], [time(12000, Modifier.DNF)]]) {
			const trend = computeTimeTrend(times);
			expect(trend.minMs).toBeNull();
			expect(trend.maxMs).toBeNull();
			expect(trend.dnfCount).toBe(times.length);
			expect(trend.yMin).toBe(0);
			expect(trend.yMax).toBe(0);
		}
	});

	test("preserves sampling boundaries and always includes the last solve", () => {
		for (const count of [1, 99, 100, 101, 199, 200, 201, 1000]) {
			const times = Array.from({ length: count }, () => time(12000));
			const { chartData } = computeTimeTrend(times);
			const step = Math.ceil(count / 100);
			const expectedSolves = Array.from(
				{ length: Math.ceil(count / step) },
				(_, index) => index * step + 1,
			);
			if (expectedSolves.at(-1) !== count) expectedSolves.push(count);
			expect(chartData.map((point) => point.solve)).toEqual(expectedSolves);
			expect(chartData.length).toBeLessThanOrEqual(101);
		}
	});

	test("uses unsampled solves for bounds and DNF counts", () => {
		const times = Array.from({ length: 200 }, () => time(12000));
		times[1] = time(1000);
		times[3] = time(30000, Modifier.PlusTwo);
		times[5] = time(12000, Modifier.DNF);
		const trend = computeTimeTrend(times);
		expect(trend.chartData.some((point) => point.solve === 2)).toBe(false);
		expect(trend.minMs).toBe(1000);
		expect(trend.maxMs).toBe(32000);
		expect(trend.dnfCount).toBe(1);
		expect(trend.yMin).toBe(0);
		expect(trend.yMax).toBe(34480);
	});

	test("handles histories larger than JavaScript's argument limit", () => {
		const times = Array.from({ length: 200000 }, () => time(12000));
		const trend = computeTimeTrend(times);
		expect(trend.chartData.length).toBe(101);
		expect(trend.chartData.at(-1)?.solve).toBe(times.length);
		expect(trend.minMs).toBe(12000);
		expect(trend.maxMs).toBe(12000);
	});
});
