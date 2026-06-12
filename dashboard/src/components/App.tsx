import {useEffect, useState} from "react";
import type {History} from "../types/types";
import {usePersistedSessions} from "../utils/usePersistedSessions";
import {parseHistoryFile} from "../utils/parse";
import {Header} from "./Header.tsx";
import {DropImport} from "./DropImport.tsx";
import {SessionSelection} from "./SessionSelection.tsx";
import {TimerDisplay} from "./TimerDisplay.tsx";
import TimesColumn from "./TimesColumn.tsx";

function App() {
    const [sessions, setSessions] = usePersistedSessions();
    const [selectedIndex, setSelectedIndex] = useState(0);

    useEffect(() => {
        fetch("/api/sessions")
            .then((res) => {
                if (!res.ok) throw new Error(`HTTP ${res.status}`);
                return res.text();
            })
            .then((text) => {
                const parsed = parseHistoryFile(text);
                if (parsed.length > 0) {
                    setSessions(parsed);
                }
            })
            .catch(() => {
            });
    }, [setSessions]);

    useEffect(() => {
        if (sessions.length === 0) {
            setSelectedIndex(0);
            return;
        }
        if (selectedIndex >= sessions.length) {
            setSelectedIndex(sessions.length - 1);
        }
    }, [sessions, selectedIndex]);

    function handleImport(nextSessions: History[]) {
        setSessions(nextSessions);
        setSelectedIndex(0);
    }

    const selectedSession = sessions[selectedIndex];

    return (
        <div className="min-h-screen bg-matte transition-colors duration-300">
            <Header showImport={sessions.length > 0} setSessions={handleImport} />
            <main className="max-w-6xl mx-auto px-4 sm:px-6 py-6 sm:py-8">
                {sessions.length === 0 ? (
                    <div className="flex flex-col items-center justify-center gap-5 mt-20 sm:mt-28 text-center animate-fade-in-up">
                        <TimerDisplay ms={null} size="hero" label="No session loaded" fallback="00:00.000" />
                        <p className="text-sm text-muted max-w-xs">
                            Import your <code className="font-mono text-xs bg-raised px-1 py-0.5">times.json</code> to review solves, averages, and trends.
                        </p>
                        <DropImport onImport={handleImport} />
                    </div>
                ) : (
                    <div className="animate-fade-in-up">
                        <SessionSelection sessions={sessions} selectedIndex={selectedIndex} onSelect={setSelectedIndex} />
                        {selectedSession && <TimesColumn history={selectedSession} />}
                    </div>
                )}
            </main>
        </div>
    );
}

export default App;
