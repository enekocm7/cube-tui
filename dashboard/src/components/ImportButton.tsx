import {Upload} from "lucide-react";
import * as React from "react";
import {useRef} from "react";
import type {History} from "../types/types";
import {parseHistoryFile} from "../utils/parse.ts";

interface ImportButtonProps {
	onImport: (sessions: History[]) => void;
	compact?: boolean;
}

export function ImportButton({ onImport, compact = false }: ImportButtonProps) {
	const fileInputRef = useRef<HTMLInputElement>(null);

	function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
		const file = e.target.files?.[0];
		if (!file) return;
		const reader = new FileReader();
		reader.onload = () => {
			const sessions = parseHistoryFile(reader.result as string);
			onImport(sessions);

			e.target.value = "";
		};
		reader.readAsText(file);
	}

	return (
		<>
			<input
				ref={fileInputRef}
				type="file"
				accept=".json,application/json"
				className="hidden"
				onChange={handleFileChange}
			/>
			<button
				onClick={() => fileInputRef.current?.click()}
				aria-label="Import history JSON"
				className={
					compact
						? "flex items-center justify-center rounded-full w-8 h-8 border border-border text-text-muted hover:text-text hover:border-border-hover transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
						: "flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium bg-btn-bg border border-border text-text-muted hover:text-text hover:border-border-hover transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
				}
			>
				<Upload size={compact ? 14 : 15} strokeWidth={2} />
				{!compact && "Import times.json"}
			</button>
		</>
	);
}
