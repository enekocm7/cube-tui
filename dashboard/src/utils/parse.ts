import type { History, Time } from "../types/types";
import { Modifier, WcaEvent } from "../types/types";

/** Narrows an unknown value to a supported puzzle event. */
function isWcaEvent(value: unknown): value is WcaEvent {
	return (
		typeof value === "string" &&
		Object.values(WcaEvent).includes(value as WcaEvent)
	);
}

/** Narrows an unknown value to a supported solve modifier. */
function isModifier(value: unknown): value is Modifier {
	return (
		typeof value === "string" &&
		Object.values(Modifier).includes(value as Modifier)
	);
}

/** Validates and normalizes one persisted solve record. */
function parseTime(raw: unknown): Time {
	if (typeof raw !== "object" || raw === null)
		throw new Error("Invalid time entry");

	const t = raw as Record<string, unknown>;

	if (typeof t.timestamp_in_millis !== "number")
		throw new Error("Missing timestamp_in_millis");
	if (!isWcaEvent(t.event))
		throw new Error(`Unknown event: ${String(t.event)}`);
	if (typeof t.scramble !== "string") throw new Error("Missing scramble");

	return {
		timestamp_in_millis: t.timestamp_in_millis,
		event: t.event,
		scramble: t.scramble,
		solved_at_unix_ms:
			typeof t.solved_at_unix_ms === "number" ? t.solved_at_unix_ms : 0,
		modifier: isModifier(t.modifier) ? t.modifier : Modifier.None,
	};
}

/** Validates one persisted session and all of its solves. */
function parseHistory(raw: unknown): History {
	if (typeof raw !== "object" || raw === null)
		throw new Error("Invalid history object");

	const h = raw as Record<string, unknown>;

	if (!Array.isArray(h.times)) throw new Error("Missing times array");

	return { times: h.times.map(parseTime) };
}

/** Parses the JSON history format exported by the TUI. */
export function parseHistoryFile(json: string): History[] {
	const raw: unknown = JSON.parse(json);

	if (!Array.isArray(raw)) throw new Error("Expected an array of sessions");

	return raw.map(parseHistory);
}
