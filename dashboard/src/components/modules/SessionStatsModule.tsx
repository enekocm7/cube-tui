import { formatStat } from "../../utils/format";

interface StatCardProps {
    label: string;
    value: string;
}

function StatCard({ label, value }: StatCardProps) {
    const dim = value === "—" || value === "DNF";

    return (
        <div className="bg-raised rounded-lg px-3 py-2.5 border border-border flex flex-col gap-1 min-w-0">
            <span className="text-[10px] uppercase tracking-widest text-text-dim font-semibold">{label}</span>
            <span className={`font-mono text-[15px] leading-tight font-semibold tabular-nums whitespace-nowrap ${dim ? "text-text-muted" : "text-text"}`}>
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

export function SessionStatsModule({ eventLabel, timesCount, bestMs, ao5, ao12, meanMs }: SessionStatsModuleProps) {
    return (
        <section className="rounded-xl border border-border bg-surface animate-fade-in-up shadow-lg p-5">
            <div className="flex items-baseline justify-between mb-4">
                <h2 className="text-base font-semibold text-text tracking-tight">{eventLabel}</h2>
                <span className="text-xs text-text-muted">
                    <span className="font-mono font-semibold text-accent">{timesCount}</span> solve{timesCount !== 1 ? "s" : ""}
                </span>
            </div>

            <div className="grid grid-cols-2 gap-2.5">
                <StatCard label="Best" value={formatStat(bestMs)} />
                <StatCard label="Ao5" value={formatStat(ao5)} />
                <StatCard label="Ao12" value={formatStat(ao12)} />
                <StatCard label="Mean" value={formatStat(meanMs)} />
            </div>
        </section>
    );
}
