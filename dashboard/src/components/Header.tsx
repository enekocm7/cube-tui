import {Moon, Sun} from "lucide-react";
import {useEffect, useState} from "react";
import {CubeLogo} from "./CubeLogo.tsx";

export function Header() {
    const [darkMode, setDarkMode] = useState(true);

    useEffect(() => {
        const root = document.documentElement;
        if (darkMode) {
            root.classList.add("dark");
        } else {
            root.classList.remove("dark");
        }
    }, [darkMode]);

    return (
        <header
            className="sticky top-0 z-50 w-full border-b border-border bg-bg-dark/75 backdrop-blur-xl transition-colors duration-300">
            <div className="max-w-5xl mx-auto px-6 h-16 flex items-center justify-between">
                <div className="flex items-center gap-3">
                    <CubeLogo size={30}/>
                    <span
                        className="font-bold text-base tracking-tight transition-colors duration-300"
                        style={{color: darkMode ? "#ffffff" : "#1a1a2e"}}
                    >
                        cube<span style={{color: "#7c6eff"}}>tui</span>
                    </span>
                </div>

                <button
                    onClick={() => setDarkMode(prev => !prev)}
                    aria-label="Toggle theme"
                    role="switch"
                    aria-checked={darkMode}
                    className="relative flex items-center gap-1.5 rounded-full px-1 py-1 border transition-all duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                    style={{
                        width: "5.5rem",
                        background: darkMode ? "#1c1c2e" : "#e8e8f4",
                        borderColor: darkMode ? "#1e1e32" : "#c8c8e0",
                    }}
                >
                    <span
                        className="absolute top-1 bottom-1 rounded-full transition-all duration-300 shadow-sm"
                        style={{
                            width: "calc(50% - 4px)",
                            left: darkMode ? "4px" : "calc(50%)",
                            background: darkMode ? "#2c2c46" : "#ffffff",
                        }}
                    />

                    <span
                        className="relative z-10 flex items-center justify-center w-1/2 transition-colors duration-300"
                        style={{color: darkMode ? "#7c6eff" : "#9090b0"}}
                    >
                        <Moon size={13} strokeWidth={2}/>
                    </span>

                    <span
                        className="relative z-10 flex items-center justify-center w-1/2 transition-colors duration-300"
                        style={{color: darkMode ? "#9090b0" : "#f59e0b"}}
                    >
                        <Sun size={13} strokeWidth={2}/>
                    </span>
                </button>

            </div>
        </header>
    );
}