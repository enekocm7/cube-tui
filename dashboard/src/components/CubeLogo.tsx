interface CubeLogoProps {
    size?: number;
}

export function CubeLogo({ size = 32 }: CubeLogoProps) {
    return <img src="/trace.svg" width={size} height={size} alt="Cube-TUI logo" />;
}
