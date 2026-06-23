import { formatStat } from "../../utils/format";

interface RecordsModuleProps {
	bestSingle: number | null;
	bestAo5: number | "DNF" | null;
	bestAo12: number | "DNF" | null;
	bestAo50: number | "DNF" | null;
	bestAo100: number | "DNF" | null;
}

interface RecordCardProps {
	label: string;
	value: string;
}

function RecordCard({ label, value }: RecordCardProps) {
	const dim = value === "—" || value === "DNF";

	return (
		<div className="bg-cell border border-border px-3 py-2.5 flex flex-col gap-0.5">
			<span className="text-[10px] uppercase tracking-[0.12em] text-muted font-semibold">
				{label}
			</span>
			<span
				className={`font-mono text-sm font-medium tabular-nums ${dim ? "text-muted" : "text-ink"}`}
			>
				{value}
			</span>
		</div>
	);
}

export function RecordsModule({
	bestSingle,
	bestAo5,
	bestAo12,
	bestAo50,
	bestAo100,
}: RecordsModuleProps) {
	return (
		<section className="h-full border border-border bg-surface animate-fade-in-up p-4 sm:p-5 flex flex-col">
			<div className="flex items-baseline justify-between mb-4">
				<h2 className="text-sm font-medium text-ink tracking-tight">
					Session records
				</h2>
			</div>

			<div className="grid grid-cols-2 gap-2 flex-1">
				<RecordCard label="Best single" value={formatStat(bestSingle)} />
				<RecordCard label="Best Ao5" value={formatStat(bestAo5)} />
				<RecordCard label="Best Ao12" value={formatStat(bestAo12)} />
				<RecordCard label="Best Ao50" value={formatStat(bestAo50)} />
				<RecordCard label="Best Ao100" value={formatStat(bestAo100)} />
			</div>
		</section>
	);
}
