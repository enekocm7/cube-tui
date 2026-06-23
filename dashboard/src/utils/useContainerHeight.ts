import { type RefObject, useEffect, useState } from "react";

export function useContainerHeight(
	ref: RefObject<HTMLElement | null>,
	defaultHeight: number,
): number {
	const [height, setHeight] = useState(defaultHeight);

	useEffect(() => {
		const el = ref.current;
		if (!el) return;

		const update = () => {
			setHeight(el.clientHeight);
		};

		update();

		const observer = new ResizeObserver(update);
		observer.observe(el);

		window.addEventListener("resize", update);
		return () => {
			observer.disconnect();
			window.removeEventListener("resize", update);
		};
	}, [ref]);

	return height;
}
