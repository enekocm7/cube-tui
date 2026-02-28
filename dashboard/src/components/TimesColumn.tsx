import { useEffect, useState } from "react"
import { Hash, X } from "lucide-react"
import type { History, Time, WcaEvent } from "../types/types"
import { Modifier, WCA_EVENT_NAMES } from "../types/types"
import {
    effectiveMs,
    formatMillis,
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
    onOpen: () => void
    animationDelay: number
}

function TimeRow({ index, time, isBest, onOpen, animationDelay }: TimeRowProps) {
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
            `}
            style={{ animationDelay: `${animationDelay}ms`, animationFillMode: "both" }}
            onClick={onOpen}
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

                <span className="text-[10px] uppercase tracking-widest text-text-dim font-semibold shrink-0">
                    Details
                </span>
            </div>
        </div>
    )
}

function formatDateTime(unix_ms: number): string {
    if (!unix_ms) return "—"
    return new Date(unix_ms).toLocaleString("en", { dateStyle: "medium", timeStyle: "short" })
}

function modifierLabel(modifier: Modifier): string {
    switch (modifier) {
        case Modifier.None:
            return "None"
        case Modifier.PlusTwo:
            return "+2"
        case Modifier.DNF:
            return "DNF"
    }
}

function formatDelta(ms: number): string {
    const sign = ms >= 0 ? "+" : "-"
    return `${sign}${formatMillis(Math.abs(ms))}`
}

interface TimeDetailsModalProps {
    time: Time
    solveIndex: number
    isBest: boolean
    bestMs: number | null
    onClose: () => void
}

function TimeDetailsModal({ time, solveIndex, isBest, bestMs, onClose }: TimeDetailsModalProps) {
    const rawMs = time.timestamp_in_millis
    const finalMs = effectiveMs(time)
    const deltaVsBest = finalMs !== null && bestMs !== null ? finalMs - bestMs : null

    return (
        <div
            className="fixed inset-0 z-50 bg-black/55 backdrop-blur-sm flex items-center justify-center px-4"
            onClick={onClose}
        >
            <div
                className="w-full max-w-xl rounded-xl border border-border bg-surface shadow-lg"
                onClick={(e) => e.stopPropagation()}
                role="dialog"
                aria-modal="true"
                aria-label="Solve details"
            >
                <div className="flex items-center justify-between px-5 py-4 border-b border-border">
                    <div>
                        <p className="text-xs uppercase tracking-widest text-text-dim font-semibold">Solve {solveIndex}</p>
                        <h3 className="text-base font-semibold text-text mt-1">{formatTime(time)}</h3>
                    </div>
                    <button
                        onClick={onClose}
                        aria-label="Close details"
                        className="rounded-lg border border-border text-text-muted hover:text-text hover:border-border-hover p-1.5 transition-colors"
                    >
                        <X size={16} strokeWidth={2} />
                    </button>
                </div>

                <div className="px-5 py-4 grid grid-cols-2 gap-2">
                    <StatCard label="Event" value={WCA_EVENT_NAMES[time.event]} />
                    <StatCard label="Penalty" value={modifierLabel(time.modifier)} />
                    <StatCard label="Raw" value={formatMillis(rawMs)} />
                    <StatCard label="Final" value={formatTime(time)} />
                    <StatCard label="Session Best" value={isBest ? "Yes" : "No"} />
                    <StatCard label="Δ vs Best" value={deltaVsBest === null ? "—" : formatDelta(deltaVsBest)} />
                </div>

                <div className="px-5 pb-5">
                    <p className="text-[10px] uppercase tracking-widest text-text-dim font-semibold mb-1.5">Solved At</p>
                    <p className="text-sm text-text-muted mb-4">{formatDateTime(time.solved_at_unix_ms)}</p>

                    <p className="text-[10px] uppercase tracking-widest text-text-dim font-semibold mb-1.5">Scramble</p>
                    <div className="rounded-lg border border-border bg-raised px-3 py-2.5 text-xs text-text-muted font-mono leading-relaxed break-all">
                        {time.scramble || "—"}
                    </div>
                </div>
            </div>
        </div>
    )
}

interface TimesColumnProps {
    history: History
}

export default function TimesColumn({ history }: TimesColumnProps) {
    const [selectedIdx, setSelectedIdx] = useState<number | null>(null)

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

    useEffect(() => {
        function handleEscape(event: KeyboardEvent) {
            if (event.key === "Escape") {
                setSelectedIdx(null)
            }
        }

        window.addEventListener("keydown", handleEscape)
        return () => window.removeEventListener("keydown", handleEscape)
    }, [])

    function handleOpen(i: number) {
        setSelectedIdx(i)
    }

    const selectedTime = selectedIdx !== null ? reversed[selectedIdx] : null
    const selectedSolveIndex = selectedIdx !== null ? times.length - selectedIdx : 0
    const selectedEffective = selectedTime ? effectiveMs(selectedTime) : null
    const selectedIsBest = selectedEffective !== null && selectedEffective === bestMs

    return (
        <>
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
                                onOpen={() => handleOpen(i)}
                                animationDelay={Math.min(i * 18, 400)}
                            />
                        )
                    })}
                </div>
            )}
            </div>

            {selectedTime && (
                <TimeDetailsModal
                    time={selectedTime}
                    solveIndex={selectedSolveIndex}
                    isBest={selectedIsBest}
                    bestMs={bestMs}
                    onClose={() => setSelectedIdx(null)}
                />
            )}
        </>
    )
}
