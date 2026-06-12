import {CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis} from "recharts";
import type {Time} from "../../types/types";
import {effectiveMs, formatMillis} from "../../utils/format";

interface TimeTrendModuleProps {
    times: Time[];
}

const MAX_CHART_POINTS = 250;

function downsample<T>(items: T[], target: number): T[] {
    if (items.length <= target) return items;
    const step = Math.ceil(items.length / target);
    const sampled: T[] = [];
    for (let i = 0; i < items.length; i += step) {
        sampled.push(items[i]);
    }
    // Always include the last point so the trend reaches the present solve.
    if (sampled[sampled.length - 1] !== items[items.length - 1]) {
        sampled.push(items[items.length - 1]);
    }
    return sampled;
}

export function TimeTrendModule({times}: TimeTrendModuleProps) {
    const allPoints = times.map((time, index) => ({
        solve: index + 1,
        ms: effectiveMs(time),
    }));

    const chartData = downsample(allPoints, MAX_CHART_POINTS);

    const validTimes = allPoints.map((point) => point.ms).filter((ms): ms is number => ms !== null);
    const dnfCount = allPoints.length - validTimes.length;

    if (validTimes.length === 0) {
        return (
            <section className="border border-border bg-surface animate-fade-in-up p-5">
                <p className="text-[10px] uppercase tracking-[0.14em] text-muted font-semibold mb-2">Time trend</p>
                <p className="text-sm text-muted">No valid solves to chart.</p>
            </section>
        );
    }

    const minMs = Math.min(...validTimes);
    const maxMs = Math.max(...validTimes);
    const rangePadding = Math.max(1000, Math.round((maxMs - minMs) * 0.08));
    const yMin = Math.max(0, minMs - rangePadding);
    const yMax = maxMs + rangePadding;

    return (
        <section className="border border-border bg-surface animate-fade-in-up p-5">
            <div className="flex items-center justify-between mb-3">
                <p className="text-[10px] uppercase tracking-[0.14em] text-muted font-semibold">Time trend</p>
                <span className="text-xs text-muted">
                    <span className="font-mono tabular-nums">{allPoints.length}</span> solve{allPoints.length === 1 ? "" : "s"} ·{" "}
                    <span className="font-mono tabular-nums">{dnfCount}</span> DNF
                    {chartData.length < allPoints.length && (
                        <>
                            {" · "}
                            <span className="font-mono tabular-nums">{chartData.length}</span> shown
                        </>
                    )}
                </span>
            </div>

            <div className="h-56">
                <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={chartData} margin={{top: 8, right: 8, bottom: 8, left: 8}}>
                        <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" vertical={false} />
                        <XAxis
                            dataKey="solve"
                            tick={{fill: "var(--muted)", fontSize: 10, fontFamily: "var(--font-mono)"}}
                            tickLine={false}
                            axisLine={{stroke: "var(--border)"}}
                            label={{
                                value: "Solve #",
                                position: "insideBottom",
                                offset: -4,
                                fill: "var(--muted)",
                                fontSize: 10,
                                fontFamily: "var(--font-mono)",
                            }}
                        />
                        <YAxis
                            domain={[yMin, yMax]}
                            tick={{fill: "var(--muted)", fontSize: 10, fontFamily: "var(--font-mono)"}}
                            tickLine={false}
                            axisLine={{stroke: "var(--border)"}}
                            tickFormatter={(value) => formatMillis(value as number)}
                            width={64}
                        />
                        <Tooltip
                            cursor={{stroke: "var(--border-hover)", strokeWidth: 1}}
                            contentStyle={{
                                background: "var(--surface)",
                                border: "1px solid var(--border)",
                                borderRadius: 0,
                                color: "var(--ink)",
                                fontFamily: "var(--font-mono)",
                                fontSize: 12,
                            }}
                            formatter={(value) => formatMillis(value as number)}
                            labelFormatter={(label) => `Solve ${label}`}
                        />
                        <Line
                            type="monotone"
                            dataKey="ms"
                            connectNulls={false}
                            stroke="var(--accent)"
                            strokeWidth={2}
                            dot={{r: 2, fill: "var(--accent)", strokeWidth: 0}}
                            activeDot={{r: 4, fill: "var(--accent)"}}
                        />
                    </LineChart>
                </ResponsiveContainer>
            </div>

            <div className="mt-2 flex items-center justify-between text-[10px] text-muted font-mono tabular-nums">
                <span>{formatMillis(minMs)}</span>
                <span>{formatMillis(maxMs)}</span>
            </div>
        </section>
    );
}
