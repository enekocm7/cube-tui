import {useEffect, useMemo, useState} from "react";
import {formatMillis} from "../utils/format";

interface TimerDisplayProps {
    ms: number | null;
    label?: string;
    animate?: boolean;
    size?: "hero" | "large" | "small";
    fallback?: string;
}

function useSettledValue(target: number | null, enabled: boolean): number | null {
    const prefersReducedMotion = useMemo(() => {
        if (typeof window === "undefined") return true;
        return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    }, []);

    const shouldAnimate = enabled && !prefersReducedMotion && target !== null && target > 0;
    const [display, setDisplay] = useState(shouldAnimate ? 0 : target);

    useEffect(() => {
        if (!shouldAnimate) {
            setDisplay(target);
            return;
        }

        const duration = 600;
        const start = performance.now();
        const from = 0;
        const to = target;

        let raf = 0;
        const step = (now: number) => {
            const t = Math.min((now - start) / duration, 1);
            const eased = 1 - Math.pow(1 - t, 3);
            setDisplay(Math.round(from + (to - from) * eased));
            if (t < 1) {
                raf = requestAnimationFrame(step);
            }
        };

        raf = requestAnimationFrame(step);
        return () => cancelAnimationFrame(raf);
    }, [target, shouldAnimate]);

    return display;
}

export function TimerDisplay({ms, label, animate = false, size = "large", fallback = "—"}: TimerDisplayProps) {
    const settled = useSettledValue(ms, animate);

    const textSize =
        size === "hero"
            ? "text-5xl sm:text-6xl md:text-7xl"
            : size === "large"
              ? "text-4xl sm:text-5xl"
              : "text-2xl sm:text-3xl";

    return (
        <div className={`flex flex-col ${size === "hero" ? "gap-2" : "gap-1"}`}>
            <span
                className={`font-mono font-medium tabular-nums tracking-tight text-ink ${textSize}`}
                aria-label={ms === null ? fallback : formatMillis(ms)}
            >
                {settled === null ? fallback : formatMillis(settled)}
            </span>
            {label && <span className="text-[10px] uppercase tracking-[0.14em] text-muted font-semibold">{label}</span>}
        </div>
    );
}
