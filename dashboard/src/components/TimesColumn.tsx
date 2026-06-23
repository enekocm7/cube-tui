import { X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { History, Time, WcaEvent } from "../types/types";
import { Modifier, WCA_EVENT_NAMES } from "../types/types";
import {
	computeAo,
	computeBest,
	computeBestAo,
	computeMean,
	computePercentile,
	effectiveMs,
	formatMillis,
	formatTime,
} from "../utils/format";
import { HistoryModule } from "./modules/HistoryModule";
import { RecordsModule } from "./modules/RecordsModule";
import { SessionStatsModule } from "./modules/SessionStatsModule";
import { TimeTrendModule } from "./modules/TimeTrendModule";
import { TimerDisplay } from "./TimerDisplay.tsx";

function ModalStatCard({ label, value }: { label: string; value: string }) {
	const dim = value === "—" || value === "DNF";
	return (
		<div className="bg-cell border border-border px-3 py-2.5 flex flex-col gap-1">
			<span className="text-[10px] uppercase tracking-[0.12em] text-muted font-semibold">
				{label}
			</span>
			<span
				className={`font-mono text-sm font-semibold tabular-nums ${dim ? "text-muted" : "text-ink"}`}
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
		<button
			type="button"
			className="fixed inset-0 z-50 bg-matte/80 backdrop-blur-sm flex items-center justify-center px-4 cursor-default"
			onClick={onClose}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") onClose();
			}}
		>
			<div
				className="w-full max-w-xl border border-border bg-surface shadow-lg"
				onClick={(e) => e.stopPropagation()}
				onKeyDown={(e) => e.stopPropagation()}
				role="dialog"
				aria-modal="true"
				aria-label="Solve details"
			>
				<div className="flex items-center justify-between px-5 py-4 border-b border-border">
					<div>
						<p className="text-[10px] uppercase tracking-[0.12em] text-muted font-semibold">
							Solve {solveIndex}
						</p>
						<h3 className="text-lg font-mono font-semibold text-ink mt-1">
							{formatTime(time)}
						</h3>
					</div>
					<button
						type="button"
						onClick={onClose}
						aria-label="Close details"
						className="border border-border text-muted hover:text-ink hover:border-border-hover p-1.5 transition-colors"
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
					<p className="text-[10px] uppercase tracking-[0.12em] text-muted font-semibold mb-1.5">
						Solved At
					</p>
					<p className="text-sm text-muted mb-4">
						{formatDateTime(time.solved_at_unix_ms)}
					</p>

					<p className="text-[10px] uppercase tracking-[0.12em] text-muted font-semibold mb-1.5">
						Scramble
					</p>
					<div className="border border-border bg-cell px-3 py-2.5 text-xs text-muted font-mono leading-relaxed break-all">
						{time.scramble || "—"}
					</div>
				</div>
			</div>
		</button>
	);
}

interface TimesColumnProps {
	history: History;
}

export default function TimesColumn({ history }: TimesColumnProps) {
	const [selectedIdx, setSelectedIdx] = useState<number | null>(null);

	const times = history.times;

	const {
		bestMs,
		ao5,
		ao12,
		ao50,
		ao100,
		bestAo5,
		bestAo12,
		bestAo50,
		bestAo100,
		meanMs,
		lastSolvePercentile,
		eventLabel,
		reversed,
	} = useMemo(() => {
		const reversed = [...times].reverse();
		const bestMs = computeBest(times);
		const ao5 = computeAo(times, 5);
		const ao12 = computeAo(times, 12);
		const ao50 = computeAo(times, 50);
		const ao100 = computeAo(times, 100);
		const bestAo5 = computeBestAo(times, 5);
		const bestAo12 = computeBestAo(times, 12);
		const bestAo50 = computeBestAo(times, 50);
		const bestAo100 = computeBestAo(times, 100);
		const meanMs = computeMean(times);
		const lastSolvePercentile = computePercentile(times);

		const eventCounts = times.reduce<Partial<Record<WcaEvent, number>>>(
			(acc, t) => {
				acc[t.event] = (acc[t.event] ?? 0) + 1;
				return acc;
			},
			{},
		);
		const mainEvent = (
			Object.entries(eventCounts) as [WcaEvent, number][]
		).sort((a, b) => b[1] - a[1])[0]?.[0];
		const eventLabel = mainEvent ? WCA_EVENT_NAMES[mainEvent] : "Session";

		return {
			bestMs,
			ao5,
			ao12,
			ao50,
			ao100,
			bestAo5,
			bestAo12,
			bestAo50,
			bestAo100,
			meanMs,
			lastSolvePercentile,
			eventLabel,
			reversed,
		};
	}, [times]);

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
			<div className="grid grid-cols-1 lg:grid-cols-12 gap-4 sm:gap-5">
				{/* Hero: session best */}
				<div className="lg:col-span-5">
					<section className="h-full border border-border bg-surface p-5 sm:p-6 flex flex-col justify-between animate-fade-in-up">
						<div>
							<div className="flex items-center gap-2 mb-4">
								<span className="text-xs font-semibold text-accent uppercase tracking-[0.08em]">
									{eventLabel}
								</span>
								<span className="text-xs text-muted">·</span>
								<span className="text-xs text-muted font-mono tabular-nums">
									{times.length} solve{times.length !== 1 ? "s" : ""}
								</span>
							</div>
							<TimerDisplay
								ms={bestMs}
								label="Session best"
								size="hero"
								animate={true}
							/>
						</div>
						<div className="mt-6 pt-4 border-t border-border flex items-center justify-between">
							<p className="text-xs text-muted">
								Last solve:{" "}
								<span className="font-mono tabular-nums text-ink">
									{times.length > 0 ? formatTime(times[times.length - 1]) : "—"}
								</span>
							</p>
							{lastSolvePercentile !== null && (
								<p className="text-xs text-muted">
									Faster than{" "}
									<span className="font-mono tabular-nums text-accent">
										{lastSolvePercentile}%
									</span>
								</p>
							)}
						</div>
					</section>
				</div>

				{/* Stats grid */}
				<div className="lg:col-span-7">
					<SessionStatsModule
						eventLabel={eventLabel}
						timesCount={times.length}
						bestMs={bestMs}
						ao5={ao5}
						ao12={ao12}
						ao50={ao50}
						ao100={ao100}
						meanMs={meanMs}
					/>
				</div>

				{/* Records */}
				<div className="lg:col-span-4">
					<RecordsModule
						bestSingle={bestMs}
						bestAo5={bestAo5}
						bestAo12={bestAo12}
						bestAo50={bestAo50}
						bestAo100={bestAo100}
					/>
				</div>

				{/* Time trend */}
				<div className="lg:col-span-8">
					<TimeTrendModule times={times} />
				</div>

				{/* History */}
				<div className="lg:col-span-12">
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
