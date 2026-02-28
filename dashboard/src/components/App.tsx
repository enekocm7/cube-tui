import {useState} from "react"
import {Header} from "./Header.tsx"
import {ImportButton} from "./ImportButton.tsx"
import type {History} from "../types/types"
import TimesColumn from "./TimesColumn.tsx";

function App() {
    const [sessions, setSessions] = useState<History[]>([])

    return (
        <div className="min-h-screen transition-colors duration-300">
            <Header showImport={sessions.length > 0} setSessions={setSessions}/>
            <main className="max-w-5xl mx-auto px-6 py-8">
                {sessions.length === 0 ? (
                    <div className="flex flex-col items-center justify-center gap-4 mt-24 text-center">
                        <p className="text-text-muted text-sm">No data loaded yet.</p>
                        <ImportButton onImport={setSessions}/>
                    </div>
                ) : (
                    <TimesColumn history={sessions[0]}/>
                )}
            </main>
        </div>
    )
}

export default App
