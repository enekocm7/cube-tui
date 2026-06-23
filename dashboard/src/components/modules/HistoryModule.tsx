import { Hash } from "lucide-react";
import { useRef } from "react";
import { FixedSizeList, type ListChildComponentProps } from "react-window";
import type { Time } from "../../types/types";
import { Modifier, WCA_EVENT_NAMES } from "../../types/types";
import { effectiveMs, formatDate, formatTime } from "../../utils/format";
import { useContainerHeight } from "../../utils/useContainerHeight";

const ROW_HEIGHT = 44;

interface TimeRowProps {
	index: number;
	time: Time;
	isBest: boolean;
	onOpen: () => void;
}

function TimeRow({ index, time, isBest, onOpen }: TimeRowProps) {
	const isDnf = time.modifier === Modifier.DNF;
	const isPlusTwo = time.modifier === Modifier.PlusTwo;

	const timeColor = isDnf
		? "text-bad"
		: isBest
			? "text-good"
			: isPlusTwo
				? "text-warn"
				: "text-ink";

	return (
		<button
			type="button"
			className="w-full text-left border-b border-border/40 cursor-pointer transition-colors duration-150 hover:bg-raised/70"
			onClick={onOpen}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") onOpen();
			}}
		>
			<div className="flex items-center gap-3 px-4 sm:px-5 py-2.5 h-11">
				<span className="w-8 text-right font-mono text-[11px] text-muted shrink-0 select-none">
					{index}
				</span>

				<span
					className={`font-mono text-sm font-medium tabular-nums w-32 sm:w-36 shrink-0 ${timeColor}`}
				>
					{formatTime(time)}
					{isBest && (
						<span className="ml-2 text-[9px] font-sans font-semibold uppercase tracking-widest text-good/70">
							pb
						</span>
					)}
				</span>

				<span className="hidden sm:inline-flex text-[10px] px-1.5 py-0.5 border border-border text-muted font-medium shrink-0">
					{WCA_EVENT_NAMES[time.event]}
				</span>

				<div className="flex-1" />

				<span className="text-[11px] text-muted tabular-nums font-mono">
					{formatDate(time.solved_at_unix_ms)}
				</span>
			</div>
		</button>
	);
}

interface RowData {
	times: Time[];
	reversed: Time[];
	bestMs: number | null;
	onOpenSolve: (reverseIndex: number) => void;
}

function Row({ index, style, data }: ListChildComponentProps<RowData>) {
	const { times, reversed, bestMs, onOpenSolve } = data;
	const time = reversed[index];
	const ms = effectiveMs(time);
	const isBest = ms !== null && ms === bestMs;

	return (
		<div style={style}>
			<TimeRow
				index={times.length - index}
				time={time}
				isBest={isBest}
				onOpen={() => onOpenSolve(index)}
			/>
		</div>
	);
}

interface HistoryModuleProps {
	times: Time[];
	bestMs: number | null;
	onOpenSolve: (reverseIndex: number) => void;
}

export function HistoryModule({
	times,
	bestMs,
	onOpenSolve,
}: HistoryModuleProps) {
	const reversed = [...times].reverse();
	const containerRef = useRef<HTMLDivElement>(null);
	const listHeight = useContainerHeight(containerRef, 480);

	return (
		<section className="border border-border bg-surface overflow-hidden animate-fade-in-up">
			<div className="flex items-center gap-3 px-4 sm:px-5 py-2.5 border-b border-border bg-raised/30">
				<span className="w-8 text-right">
					<Hash size={10} className="ml-auto text-muted" />
				</span>
				<span className="text-[10px] uppercase tracking-[0.14em] text-muted font-semibold w-32 sm:w-36 shrink-0">
					Time
				</span>
				<span className="hidden sm:block text-[10px] uppercase tracking-[0.14em] text-muted font-semibold">
					Event
				</span>
				<div className="flex-1" />
				<span className="text-[10px] uppercase tracking-[0.14em] text-muted font-semibold">
					Date
				</span>
				<span className="w-3" />
			</div>

			{times.length === 0 ? (
				<div className="py-20 text-center text-muted text-sm">
					No solves recorded.
				</div>
			) : (
				<div ref={containerRef} className="h-[52vh] min-h-75 max-h-160">
					<FixedSizeList
						height={listHeight}
						itemCount={reversed.length}
						itemSize={ROW_HEIGHT}
						itemData={{ times, reversed, bestMs, onOpenSolve }}
						width="100%"
					>
						{Row}
					</FixedSizeList>
				</div>
			)}
		</section>
	);
}
