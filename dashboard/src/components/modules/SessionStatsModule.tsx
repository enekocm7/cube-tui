import {formatStat} from "../../utils/format";

interface StatCardProps {
    label: string;
    value: string;
}

function StatCard({label, value}: StatCardProps) {
    const dim = value === "—" || value === "DNF";

    return (
        <div className="bg-cell border border-border px-4 py-3 flex flex-col gap-1 min-w-0">
            <span className="text-[10px] uppercase tracking-[0.12em] text-muted font-semibold">{label}</span>
            <span className={`font-mono text-xl sm:text-2xl leading-tight font-medium tabular-nums whitespace-nowrap ${dim ? "text-muted" : "text-ink"}`}>
                {value}
            </span>
        </div>
    );
}

interface SessionStatsModuleProps {
    eventLabel: string;
    timesCount: number;
    bestMs: number | null;
    ao5: number | "DNF" | null;
    ao12: number | "DNF" | null;
    meanMs: number | null;
}

export function SessionStatsModule({eventLabel, timesCount, bestMs, ao5, ao12, meanMs}: SessionStatsModuleProps) {
    return (
        <section className="h-full border border-border bg-surface animate-fade-in-up p-4 sm:p-5">
            <div className="flex items-baseline justify-between mb-4">
                <h2 className="text-sm font-medium text-ink tracking-tight">Session stats</h2>
                <span className="text-xs text-muted">
                    <span className="font-mono font-semibold text-accent">{timesCount}</span> solve{timesCount !== 1 ? "s" : ""}
                </span>
            </div>

            <div className="grid grid-cols-2 gap-2">
                <StatCard label="Best" value={formatStat(bestMs)} />
                <StatCard label="Ao5" value={formatStat(ao5)} />
                <StatCard label="Ao12" value={formatStat(ao12)} />
                <StatCard label="Mean" value={formatStat(meanMs)} />
            </div>

            <p className="mt-3 text-[10px] text-muted">Event: <span className="text-ink">{eventLabel}</span></p>
        </section>
    );
}
