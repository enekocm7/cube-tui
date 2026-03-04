import { useEffect, useState } from "react";
import type { History } from "../types/types";
import { usePersistedSessions } from "../utils/usePersistedSessions";
import { parseHistoryFile } from "../utils/parse";
import { Header } from "./Header.tsx";
import { ImportButton } from "./ImportButton.tsx";
import { SessionSelection } from "./SessionSelection.tsx";
import TimesColumn from "./TimesColumn.tsx";

function App() {
    const [sessions, setSessions] = usePersistedSessions();
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [loadedFromApi, setLoadedFromApi] = useState(false);

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
                    setLoadedFromApi(true);
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
        <div className="min-h-screen bg-bg transition-colors duration-300">
            <Header showImport={sessions.length > 0 && !loadedFromApi} setSessions={handleImport} />
            <main className="max-w-5xl mx-auto px-6 py-8">
                {sessions.length === 0 ? (
                    <div className="flex flex-col items-center justify-center gap-4 mt-24 text-center">
                        <p className="text-text-muted text-sm">No data loaded yet.</p>
                        <ImportButton onImport={handleImport} />
                    </div>
                ) : (
                    <>
                        <SessionSelection sessions={sessions} selectedIndex={selectedIndex} onSelect={setSelectedIndex} />
                        {selectedSession && <TimesColumn history={selectedSession} />}
                    </>
                )}
            </main>
        </div>
    );
}

export default App;
