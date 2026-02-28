import {useState} from "react";
import type {History} from "../types/types";
import {parseHistoryFile} from "./parse";

const STORAGE_KEY = "cube-tui:sessions";

function loadFromStorage(): History[] {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return [];
		return parseHistoryFile(raw);
	} catch {
		return [];
	}
}

export function usePersistedSessions(): [
	History[],
	(sessions: History[]) => void,
] {
	const [sessions, setSessions] = useState<History[]>(loadFromStorage);

	function setAndPersist(sessions: History[]) {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions));
		} catch {
			// storage quota exceeded or unavailable — silently ignore
		}
		setSessions(sessions);
	}

	return [sessions, setAndPersist];
}
