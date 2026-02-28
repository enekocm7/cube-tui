import {X} from "lucide-react";
import {useEffect, useState} from "react";
import type {History, Time, WcaEvent} from "../types/types";
import {Modifier, WCA_EVENT_NAMES} from "../types/types";
import {computeAo, computeBest, computeMean, effectiveMs, formatMillis, formatTime,} from "../utils/format";
import {HistoryModule} from "./modules/HistoryModule";
import {SessionStatsModule} from "./modules/SessionStatsModule";
import {TimeTrendModule} from "./modules/TimeTrendModule";

function ModalStatCard({ label, value }: { label: string; value: string }) {
	const dim = value === "—" || value === "DNF";
	return (
		<div className="bg-raised rounded-lg px-3 py-2.5 border border-border flex flex-col gap-1">
			<span className="text-[10px] uppercase tracking-widest text-text-dim font-semibold">
				{label}
			</span>
			<span
				className={`font-mono text-sm font-semibold tabular-nums ${dim ? "text-text-muted" : "text-text"}`}
			>
				{value}
			</span>
		</div>
	);
}

function formatDateTime(unix_ms: number): string {
	if (!unix_ms) return "—";
	return new Date(unix_ms).toLocaleString("en", {
		dateStyle: "medium",
		timeStyle: "short",
	});
}

function modifierLabel(modifier: Modifier): string {
	switch (modifier) {
		case Modifier.None:
			return "None";
		case Modifier.PlusTwo:
			return "+2";
		case Modifier.DNF:
			return "DNF";
	}
}

function formatDelta(ms: number): string {
	const sign = ms >= 0 ? "+" : "-";
	return `${sign}${formatMillis(Math.abs(ms))}`;
}

interface TimeDetailsModalProps {
	time: Time;
	solveIndex: number;
	isBest: boolean;
	bestMs: number | null;
	onClose: () => void;
}

function TimeDetailsModal({
	time,
	solveIndex,
	isBest,
	bestMs,
	onClose,
}: TimeDetailsModalProps) {
	const rawMs = time.timestamp_in_millis;
	const finalMs = effectiveMs(time);
	const deltaVsBest =
		finalMs !== null && bestMs !== null ? finalMs - bestMs : null;

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
						<p className="text-xs uppercase tracking-widest text-text-dim font-semibold">
							Solve {solveIndex}
						</p>
						<h3 className="text-base font-semibold text-text mt-1">
							{formatTime(time)}
						</h3>
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
					<ModalStatCard label="Event" value={WCA_EVENT_NAMES[time.event]} />
					<ModalStatCard label="Penalty" value={modifierLabel(time.modifier)} />
					<ModalStatCard label="Raw" value={formatMillis(rawMs)} />
					<ModalStatCard label="Final" value={formatTime(time)} />
					<ModalStatCard label="Session Best" value={isBest ? "Yes" : "No"} />
					<ModalStatCard
						label="Δ vs Best"
						value={deltaVsBest === null ? "—" : formatDelta(deltaVsBest)}
					/>
				</div>

				<div className="px-5 pb-5">
					<p className="text-[10px] uppercase tracking-widest text-text-dim font-semibold mb-1.5">
						Solved At
					</p>
					<p className="text-sm text-text-muted mb-4">
						{formatDateTime(time.solved_at_unix_ms)}
					</p>

					<p className="text-[10px] uppercase tracking-widest text-text-dim font-semibold mb-1.5">
						Scramble
					</p>
					<div className="rounded-lg border border-border bg-raised px-3 py-2.5 text-xs text-text-muted font-mono leading-relaxed break-all">
						{time.scramble || "—"}
					</div>
				</div>
			</div>
		</div>
	);
}

interface TimesColumnProps {
	history: History;
}

export default function TimesColumn({ history }: TimesColumnProps) {
	const [selectedIdx, setSelectedIdx] = useState<number | null>(null);

	const times = history.times;
	const reversed = [...times].reverse();

	const bestMs = computeBest(times);
	const ao5 = computeAo(times, 5);
	const ao12 = computeAo(times, 12);
	const meanMs = computeMean(times);

	const eventCounts = times.reduce<Partial<Record<WcaEvent, number>>>(
		(acc, t) => {
			acc[t.event] = (acc[t.event] ?? 0) + 1;
			return acc;
		},
		{},
	);
	const mainEvent = (Object.entries(eventCounts) as [WcaEvent, number][]).sort(
		(a, b) => b[1] - a[1],
	)[0]?.[0];
	const eventLabel = mainEvent ? WCA_EVENT_NAMES[mainEvent] : "Session";

	useEffect(() => {
		function handleEscape(event: KeyboardEvent) {
			if (event.key === "Escape") {
				setSelectedIdx(null);
			}
		}

		window.addEventListener("keydown", handleEscape);
		return () => window.removeEventListener("keydown", handleEscape);
	}, []);

	function handleOpen(i: number) {
		setSelectedIdx(i);
	}

	const selectedTime = selectedIdx !== null ? reversed[selectedIdx] : null;
	const selectedSolveIndex =
		selectedIdx !== null ? times.length - selectedIdx : 0;
	const selectedEffective = selectedTime ? effectiveMs(selectedTime) : null;
	const selectedIsBest =
		selectedEffective !== null && selectedEffective === bestMs;

	return (
		<>
			<div className="grid grid-cols-1 xl:grid-cols-12 gap-4">
				<div className="xl:col-span-4">
					<SessionStatsModule
						eventLabel={eventLabel}
						timesCount={times.length}
						bestMs={bestMs}
						ao5={ao5}
						ao12={ao12}
						meanMs={meanMs}
					/>
				</div>

				<div className="xl:col-span-8">
					<TimeTrendModule times={times} />
				</div>

				<div className="xl:col-span-12">
					<HistoryModule
						times={times}
						bestMs={bestMs}
						onOpenSolve={handleOpen}
					/>
				</div>
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
	);
}
