import {Moon, Sun} from "lucide-react";
import {useEffect, useState} from "react";
import type {History} from "../types/types";
import {CubeLogo} from "./CubeLogo.tsx";
import {ImportButton} from "./ImportButton.tsx";

interface HeaderProps {
	showImport?: boolean;
	setSessions?: (sessions: History[]) => void;
}

export function Header({ showImport = false, setSessions }: HeaderProps) {
	const [darkMode, setDarkMode] = useState(() =>
		document.documentElement.classList.contains("dark"),
	);

	useEffect(() => {
		const root = document.documentElement;
		if (darkMode) {
			root.classList.add("dark");
			localStorage.setItem("theme", "dark");
		} else {
			root.classList.remove("dark");
			localStorage.setItem("theme", "light");
		}
	}, [darkMode]);

	return (
		<header className="sticky top-0 z-50 w-full border-b border-border bg-header-bg backdrop-blur-xl transition-colors duration-300">
			<div className="max-w-5xl mx-auto px-6 h-16 flex items-center justify-between">
				<div className="flex items-center gap-3">
					<CubeLogo size={30} />
					<span className="font-bold text-base tracking-tight text-text transition-colors duration-300">
						cube<span className="text-accent">tui</span>
					</span>
				</div>

				<div className="flex items-center gap-2">
					{showImport && setSessions && (
						<ImportButton onImport={setSessions} compact={true} />
					)}
					<button
						onClick={() => setDarkMode((prev) => !prev)}
						aria-label="Toggle theme"
						role="switch"
						aria-checked={darkMode}
						data-theme={darkMode ? "dark" : "light"}
						className="theme-toggle relative flex items-center gap-1.5 rounded-full px-1 py-1 border transition-all duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
					>
						<span className="theme-toggle-thumb absolute top-1 bottom-1 rounded-full transition-all duration-300 shadow-sm" />

						<span className="theme-toggle-moon relative z-10 flex items-center justify-center w-1/2 transition-colors duration-300">
							<Moon size={13} strokeWidth={2} />
						</span>

						<span className="theme-toggle-sun relative z-10 flex items-center justify-center w-1/2 transition-colors duration-300">
							<Sun size={13} strokeWidth={2} />
						</span>
					</button>
				</div>
			</div>
		</header>
	);
}
