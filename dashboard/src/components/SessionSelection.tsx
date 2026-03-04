import type { ChangeEvent } from "react";
import type { History, WcaEvent } from "../types/types";
import { WCA_EVENT_NAMES } from "../types/types";

interface SessionSelectionProps {
    sessions: History[];
    selectedIndex: number;
    onSelect: (index: number) => void;
}

function sessionLabel(history: History, index: number): string {
    const eventCounts = history.times.reduce<Partial<Record<WcaEvent, number>>>((acc, t) => {
        acc[t.event] = (acc[t.event] ?? 0) + 1;
        return acc;
    }, {});

    const mainEvent = (Object.entries(eventCounts) as [WcaEvent, number][]).sort((a, b) => b[1] - a[1])[0]?.[0];

    const eventName = mainEvent ? WCA_EVENT_NAMES[mainEvent] : "Session";
    const solves = history.times.length;

    return `Session ${index + 1} · ${eventName} · ${solves} solve${solves === 1 ? "" : "s"}`;
}

export function SessionSelection({ sessions, selectedIndex, onSelect }: SessionSelectionProps) {
    function handleSelect(e: ChangeEvent<HTMLSelectElement>) {
        onSelect(Number(e.target.value));
    }

    return (
        <div className="mb-4 flex justify-end">
            <label className="flex items-center gap-2 text-sm text-text-muted">
                <span>Session</span>
                <select
                    value={selectedIndex}
                    onChange={handleSelect}
                    className="rounded-lg bg-btn-bg border border-border px-3 py-1.5 text-text text-sm hover:border-border-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                    aria-label="Select session"
                >
                    {sessions.map((session, index) => {
                        const key = `${session.times[0].solved_at_unix_ms}-${session.times.length}-${session.times[0]?.event ?? "none"}`;
                        return (
                            <option key={key} value={index}>
                                {sessionLabel(session, index)}
                            </option>
                        );
                    })}
                </select>
            </label>
        </div>
    );
}
