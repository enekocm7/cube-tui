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

/** Average of last `n` times (WCA rules: drop best + worst, 1 DNF allowed as worst). */
export function computeAo(times: Time[], n: number): number | "DNF" | null {
    if (times.length < n) return null;
    const last = times.slice(-n);
    const effectives = last.map(effectiveMs);
    const dnfCount = effectives.filter((ms) => ms === null).length;
    if (dnfCount > 1) return "DNF";

    const values = effectives.map((ms) => ms ?? Infinity);
    const sorted = [...values].sort((a, b) => a - b);
    const trimmed = sorted.slice(1, n - 1); // drop best and worst
    return Math.round(trimmed.reduce((a, b) => a + b, 0) / trimmed.length);
}

export function computeBest(times: Time[]): number | null {
    const effective = times.map(effectiveMs).filter((ms): ms is number => ms !== null);
    if (effective.length === 0) return null;
    return Math.min(...effective);
}

export function computeMean(times: Time[]): number | null {
    const effective = times.map(effectiveMs).filter((ms): ms is number => ms !== null);
    if (effective.length === 0) return null;
    return Math.round(effective.reduce((a, b) => a + b, 0) / effective.length);
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
