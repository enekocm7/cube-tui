import {Upload} from "lucide-react";
import type {ChangeEvent, DragEvent} from "react";
import {useRef, useState} from "react";
import type {History} from "../types/types";
import {parseHistoryFile} from "../utils/parse.ts";

interface DropImportProps {
    onImport: (sessions: History[]) => void;
}

export function DropImport({onImport}: DropImportProps) {
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [isDragging, setIsDragging] = useState(false);
    const [error, setError] = useState<string | null>(null);

    function processFile(file: File) {
        setError(null);

        if (file.type && !file.type.includes("json") && !file.name.endsWith(".json")) {
            setError("Please drop a JSON file.");
            return;
        }

        const reader = new FileReader();
        reader.onload = () => {
            try {
                const sessions = parseHistoryFile(reader.result as string);
                if (sessions.length === 0) {
                    setError("No sessions found in this file.");
                    return;
                }
                onImport(sessions);
            } catch (err) {
                setError(err instanceof Error ? err.message : "Could not parse file.");
            }
        };
        reader.onerror = () => {
            setError("Could not read file.");
        };
        reader.readAsText(file);
    }

    function handleDragOver(e: DragEvent<HTMLButtonElement>) {
        e.preventDefault();
        setIsDragging(true);
    }

    function handleDragLeave(e: DragEvent<HTMLButtonElement>) {
        e.preventDefault();
        setIsDragging(false);
    }

    function handleDrop(e: DragEvent<HTMLButtonElement>) {
        e.preventDefault();
        setIsDragging(false);

        const file = e.dataTransfer.files?.[0];
        if (file) processFile(file);
    }

    function handleFileChange(e: ChangeEvent<HTMLInputElement>) {
        const file = e.target.files?.[0];
        if (file) processFile(file);
        e.target.value = "";
    }

    return (
        <div className="w-full max-w-sm">
            <input
                ref={fileInputRef}
                type="file"
                accept=".json,application/json"
                className="hidden"
                onChange={handleFileChange}
            />
            <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                onDragOver={handleDragOver}
                onDragLeave={handleDragLeave}
                onDrop={handleDrop}
                className={`
                    w-full flex flex-col items-center justify-center gap-3
                    border-2 border-dashed px-6 py-10
                    transition-colors duration-200
                    focus:outline-none focus-visible:ring-2 focus-visible:ring-accent
                    ${isDragging
                        ? "border-accent bg-raised/60 text-ink"
                        : "border-border bg-raised/30 text-muted hover:border-border-hover hover:text-ink"
                    }
                `}
            >
                <Upload size={28} strokeWidth={1.5} />
                <span className="text-sm font-medium">
                    Drop your <span className="font-mono text-xs">times.json</span> here
                </span>
                <span className="text-xs text-muted">or click to browse</span>
            </button>
            {error && (
                <p className="mt-2 text-xs text-bad" role="alert">
                    {error}
                </p>
            )}
        </div>
    );
}
