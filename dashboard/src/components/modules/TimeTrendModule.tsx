import { LineChart, Line, ResponsiveContainer, CartesianGrid, XAxis, YAxis, Tooltip } from "recharts"
import type { Time } from "../../types/types"
import { effectiveMs, formatMillis } from "../../utils/format"

interface TimeTrendModuleProps {
    times: Time[]
}

export function TimeTrendModule({ times }: TimeTrendModuleProps) {
    const chartData = times.map((time, index) => ({
        solve: index + 1,
        ms: effectiveMs(time),
    }))

    const validTimes = chartData
        .map((point) => point.ms)
        .filter((ms): ms is number => ms !== null)

    const dnfCount = chartData.length - validTimes.length

    if (validTimes.length === 0) {
        return (
            <section className="rounded-xl border border-border bg-surface animate-fade-in-up shadow-lg p-5">
                <p className="text-[10px] uppercase tracking-widest text-text-dim font-semibold mb-2">Time Trend</p>
                <p className="text-sm text-text-muted">No valid solves to chart.</p>
            </section>
        )
    }

    const minMs = Math.min(...validTimes)
    const maxMs = Math.max(...validTimes)
    const rangePadding = Math.max(1000, Math.round((maxMs - minMs) * 0.08))
    const yMin = Math.max(0, minMs - rangePadding)
    const yMax = maxMs + rangePadding

    return (
        <section className="rounded-xl border border-border bg-surface animate-fade-in-up shadow-lg p-5">
            <div className="flex items-center justify-between mb-3">
                <p className="text-[10px] uppercase tracking-widest text-text-dim font-semibold">Time Trend</p>
                <span className="text-xs text-text-muted">
                    <span className="font-mono tabular-nums">{chartData.length}</span> solve{chartData.length === 1 ? "" : "s"} · <span className="font-mono tabular-nums">{dnfCount}</span> DNF
                </span>
            </div>

            <div className="h-52">
                <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={chartData} margin={{ top: 8, right: 8, bottom: 8, left: 8 }}>
                        <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" vertical={false} />
                        <XAxis
                            dataKey="solve"
                            tick={{ fill: "var(--text-dim)", fontSize: 10 }}
                            tickLine={false}
                            axisLine={{ stroke: "var(--border)" }}
                            label={{ value: "Solve #", position: "insideBottom", offset: -4, fill: "var(--text-dim)", fontSize: 10 }}
                        />
                        <YAxis
                            domain={[yMin, yMax]}
                            tick={{ fill: "var(--text-dim)", fontSize: 10 }}
                            tickLine={false}
                            axisLine={{ stroke: "var(--border)" }}
                            tickFormatter={(value) => formatMillis(value as number)}
                            width={64}
                        />
                        <Tooltip
                            cursor={{ stroke: "var(--border-hover)", strokeWidth: 1 }}
                            contentStyle={{
                                background: "var(--surface)",
                                border: "1px solid var(--border)",
                                borderRadius: "0.5rem",
                                color: "var(--text)",
                            }}
                            formatter={(value) => formatMillis(value as number)}
                            labelFormatter={(label) => `Solve ${label}`}
                        />
                        <Line
                            type="monotone"
                            dataKey="ms"
                            connectNulls={false}
                            stroke="var(--accent)"
                            strokeWidth={2.5}
                            dot={{ r: 2, fill: "var(--accent)", strokeWidth: 0 }}
                            activeDot={{ r: 4, fill: "var(--accent)" }}
                        />
                    </LineChart>
                </ResponsiveContainer>
            </div>

            <div className="mt-2 flex items-center justify-between text-[10px] text-text-dim font-mono tabular-nums">
                <span>{formatMillis(minMs)}</span>
                <span>{formatMillis(maxMs)}</span>
            </div>
        </section>
    )
}
