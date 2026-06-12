import type {ChangeEvent} from "react";
import type {History, WcaEvent} from "../types/types";
import {WCA_EVENT_NAMES} from "../types/types";

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

export function SessionSelection({sessions, selectedIndex, onSelect}: SessionSelectionProps) {
    function handleSelect(e: ChangeEvent<HTMLSelectElement>) {
        onSelect(Number(e.target.value));
    }

    return (
        <div className="mb-5 flex justify-end">
            <label className="flex items-center gap-2 text-xs sm:text-sm text-muted">
                <span className="hidden sm:inline">Session</span>
                <select
                    value={selectedIndex}
                    onChange={handleSelect}
                    className="bg-raised border border-border rounded-none px-3 py-1.5 text-ink text-xs sm:text-sm hover:border-border-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent transition-colors"
                    aria-label="Select session"
                >
                    {sessions.map((session, index) => {
                        const firstTime = session.times[0];
                        const key = `${index}-${firstTime?.solved_at_unix_ms ?? "empty"}-${session.times.length}-${firstTime?.event ?? "none"}`;
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
