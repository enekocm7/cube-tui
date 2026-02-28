import { Hash } from "lucide-react"
import type { Time } from "../../types/types"
import { Modifier, WCA_EVENT_NAMES } from "../../types/types"
import { effectiveMs, formatDate, formatTime } from "../../utils/format"

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
            </div>
        </div>
    )
}

interface HistoryModuleProps {
    times: Time[]
    bestMs: number | null
    onOpenSolve: (reverseIndex: number) => void
}

export function HistoryModule({ times, bestMs, onOpenSolve }: HistoryModuleProps) {
    const reversed = [...times].reverse()

    return (
        <section className="rounded-xl border border-border bg-surface overflow-hidden animate-fade-in-up shadow-lg">
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
                                onOpen={() => onOpenSolve(i)}
                                animationDelay={Math.min(i * 18, 400)}
                            />
                        )
                    })}
                </div>
            )}
        </section>
    )
}
