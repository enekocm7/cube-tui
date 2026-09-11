interface CubeLogoProps {
	size?: number;
}

/** Renders the scalable cube mark used in dashboard branding. */
export function CubeLogo({ size = 32 }: CubeLogoProps) {
	return (
		<img
			src="/trace.svg"
			width={size}
			height={size}
			alt="Cube-TUI logo"
			className="logo-img"
		/>
	);
}
