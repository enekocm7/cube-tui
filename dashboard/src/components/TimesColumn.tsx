import { useState } from "react"
import { ChevronDown, ChevronUp, Hash } from "lucide-react"
import type { History, Time, WcaEvent } from "../types/types"
import { Modifier, WCA_EVENT_NAMES } from "../types/types"
import {
    effectiveMs,
    formatTime,
    formatDate,
    formatStat,
    computeAo,
    computeBest,
    computeMean,
} from "../utils/format"

function StatCard({ label, value }: { label: string; value: string }) {
    const dim = value === "—" || value === "DNF"
    return (
        <div className="bg-raised rounded-lg px-3 py-2.5 border border-border flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-widest text-text-dim font-semibold">
                {label}
            </span>
            <span className={`font-mono text-sm font-semibold tabular-nums ${dim ? "text-text-muted" : "text-text"}`}>
                {value}
            </span>
        </div>
    )
}

interface TimeRowProps {
    index: number
    time: Time
    isBest: boolean
    expanded: boolean
    onToggle: () => void
    animationDelay: number
}

function TimeRow({ index, time, isBest, expanded, onToggle, animationDelay }: TimeRowProps) {
    const isDnf = time.modifier === Modifier.DNF
    const isPlusTwo = time.modifier === Modifier.PlusTwo

    const timeColor = isDnf
        ? "text-red-400"
        : isBest
            ? "text-accent"
            : isPlusTwo
                ? "text-amber-400"
                : "text-text"

    return (
        <div
            className={`
                border-b border-border/40 cursor-pointer
                transition-colors duration-150
                hover:bg-raised/70
                animate-fade-in-up
                ${expanded ? "bg-raised/50" : ""}
            `}
            style={{ animationDelay: `${animationDelay}ms`, animationFillMode: "both" }}
            onClick={onToggle}
        >
            <div className="flex items-center gap-3 px-5 py-2.5">
                <span className="w-8 text-right font-mono text-xs text-text-dim shrink-0 select-none">
                    {index}
                </span>

                <span className={`font-mono text-sm font-medium tabular-nums w-36 shrink-0 ${timeColor}`}>
                    {formatTime(time)}
                    {isBest && (
                        <span className="ml-2 text-[9px] font-sans font-bold uppercase tracking-widest text-accent/60">
                            pb
                        </span>
                    )}
                </span>

                <span className="hidden sm:inline-flex text-[10px] px-1.5 py-0.5 rounded-md bg-btn-bg border border-border text-text-dim font-medium shrink-0">
                    {WCA_EVENT_NAMES[time.event]}
                </span>

                <div className="flex-1" />

                <span className="text-xs text-text-dim tabular-nums">
                    {formatDate(time.solved_at_unix_ms)}
                </span>

                <span className="text-text-dim shrink-0 transition-transform duration-200">
                    {expanded
                        ? <ChevronUp size={12} strokeWidth={2} />
                        : <ChevronDown size={12} strokeWidth={1.5} />
                    }
                </span>
            </div>

            {expanded && time.scramble && (
                <div className="px-5 pb-3 ml-11 text-xs text-text-muted font-mono leading-relaxed break-all">
                    {time.scramble}
                </div>
            )}
        </div>
    )
}

interface TimesColumnProps {
    history: History
}

export default function TimesColumn({ history }: TimesColumnProps) {
    const [expandedIdx, setExpandedIdx] = useState<number | null>(null)

    const times = history.times
    const reversed = [...times].reverse()

    const bestMs  = computeBest(times)
    const ao5     = computeAo(times, 5)
    const ao12    = computeAo(times, 12)
    const meanMs  = computeMean(times)

    const eventCounts = times.reduce<Partial<Record<WcaEvent, number>>>((acc, t) => {
        acc[t.event] = (acc[t.event] ?? 0) + 1
        return acc
    }, {})
    const mainEvent = (Object.entries(eventCounts) as [WcaEvent, number][])
        .sort((a, b) => b[1] - a[1])[0]?.[0]
    const eventLabel = mainEvent ? WCA_EVENT_NAMES[mainEvent] : "Session"

    function handleToggle(i: number) {
        setExpandedIdx(prev => (prev === i ? null : i))
    }

    return (
        <div className="rounded-xl border border-border bg-surface overflow-hidden animate-fade-in-up shadow-lg">

            <div className="px-5 pt-5 pb-4 border-b border-border">
                <div className="flex items-baseline justify-between mb-4">
                    <h2 className="text-base font-semibold text-text tracking-tight">
                        {eventLabel}
                    </h2>
                    <span className="text-xs text-text-muted">
                        <span className="font-mono font-semibold text-accent">{times.length}</span>
                        {" "}solve{times.length !== 1 ? "s" : ""}
                    </span>
                </div>

                <div className="grid grid-cols-4 gap-2">
                    <StatCard label="Best"  value={formatStat(bestMs)} />
                    <StatCard label="Ao5"   value={formatStat(ao5)}    />
                    <StatCard label="Ao12"  value={formatStat(ao12)}   />
                    <StatCard label="Mean"  value={formatStat(meanMs)} />
                </div>
            </div>

            <div className="flex items-center gap-3 px-5 py-2 border-b border-border bg-raised/40">
                <span className="w-8 text-right">
                    <Hash size={10} className="ml-auto text-text-dim" />
                </span>
                <span className="text-[10px] uppercase tracking-widest text-text-dim font-semibold w-36 shrink-0">Time</span>
                <span className="hidden sm:block text-[10px] uppercase tracking-widest text-text-dim font-semibold">Event</span>
                <div className="flex-1" />
                <span className="text-[10px] uppercase tracking-widest text-text-dim font-semibold">Date</span>
                <span className="w-3" />
            </div>

            {times.length === 0 ? (
                <div className="py-20 text-center text-text-muted text-sm">
                    No solves recorded.
                </div>
            ) : (
                <div className="overflow-y-auto max-h-[58vh]">
                    {reversed.map((time, i) => {
                        const ms = effectiveMs(time)
                        const isBest = ms !== null && ms === bestMs
                        return (
                            <TimeRow
                                key={times.length - 1 - i}
                                index={times.length - i}
                                time={time}
                                isBest={isBest}
                                expanded={expandedIdx === i}
                                onToggle={() => handleToggle(i)}
                                animationDelay={Math.min(i * 18, 400)}
                            />
                        )
                    })}
                </div>
            )}
        </div>
    )
}
