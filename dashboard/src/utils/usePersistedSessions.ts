import { useCallback, useState } from "react";
import type { History } from "../types/types";
import { parseHistoryFile } from "./parse";

const STORAGE_KEY = "cube-tui:sessions";

/** Reads and validates sessions cached in browser local storage. */
function loadFromStorage(): History[] {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return [];
		return parseHistoryFile(raw);
	} catch {
		return [];
	}
}

/** Provides session state that is automatically synchronized to local storage. */
export function usePersistedSessions(): [
	History[],
	(sessions: History[]) => void,
] {
	const [sessions, setSessions] = useState<History[]>(loadFromStorage);

	/** Updates React state and its local-storage mirror atomically. */
	const setAndPersist = useCallback((sessions: History[]) => {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions));
		} catch {
			// storage quota exceeded or unavailable — silently ignore
		}
		setSessions(sessions);
	}, []);

	return [sessions, setAndPersist];
}
