import { CSSProperties, useMemo } from "react";
import "./orb.css";

export type OrbState = "idle" | "talking" | "listening";

interface OrbPalette {
    mainBgStart: string;
    mainBgEnd: string;
    shapeAStart: string;
    shapeAEnd: string;
    shapeBStart: string;
    shapeBMiddle: string;
    shapeBEnd: string;
    shapeCStart: string;
    shapeCMiddle: string;
    shapeCEnd: string;
    shapeDStart: string;
    shapeDMiddle: string;
    shapeDEnd: string;
    shadowColor1: string;
    shadowColor2: string;
    shadowColor3: string;
    shadowColor4: string;
    speed: number;
    hueRotation: number;
}

interface OrbProps {
    state: OrbState;
    level?: number;
    className?: string;
}

interface SvgElementsProps {
    color1: string;
    color2: string;
}

const PALETTES: Record<Exclude<OrbState, "listening">, OrbPalette> = {
    idle: {
        mainBgStart: "#6b7280",
        mainBgEnd: "#374151",
        shapeAStart: "#9ca3af",
        shapeAEnd: "#4b5563",
        shapeBStart: "#d1d5db",
        shapeBMiddle: "#9ca3af",
        shapeBEnd: "#6b7280",
        shapeCStart: "#f3f4f6",
        shapeCMiddle: "#e5e7eb",
        shapeCEnd: "#9ca3af",
        shapeDStart: "#e5e7eb",
        shapeDMiddle: "#d1d5db",
        shapeDEnd: "#9ca3af",
        shadowColor1: "rgba(107, 114, 128, 0.2)",
        shadowColor2: "rgba(75, 85, 99, 0.3)",
        shadowColor3: "rgba(156, 163, 175, 0.1)",
        shadowColor4: "rgba(209, 213, 219, 0.2)",
        speed: 0.3,
        hueRotation: 15,
    },
    talking: {
        mainBgStart: "#3b82f6",
        mainBgEnd: "#1e40af",
        shapeAStart: "#60a5fa",
        shapeAEnd: "#2563eb",
        shapeBStart: "#93c5fd",
        shapeBMiddle: "#60a5fa",
        shapeBEnd: "#3b82f6",
        shapeCStart: "#dbeafe",
        shapeCMiddle: "#bfdbfe",
        shapeCEnd: "#60a5fa",
        shapeDStart: "#bfdbfe",
        shapeDMiddle: "#93c5fd",
        shapeDEnd: "#60a5fa",
        shadowColor1: "rgba(59, 130, 246, 0.4)",
        shadowColor2: "rgba(37, 99, 235, 0.5)",
        shadowColor3: "rgba(96, 165, 250, 0.2)",
        shadowColor4: "rgba(147, 197, 253, 0.3)",
        speed: 0.8,
        hueRotation: 30,
    },
};

function getListeningPalette(level: number): OrbPalette {
    const intensity = 0.5 + level * 0.5;
    return {
        mainBgStart: "#22c55e",
        mainBgEnd: "#15803d",
        shapeAStart: "#4ade80",
        shapeAEnd: "#16a34a",
        shapeBStart: "#86efac",
        shapeBMiddle: "#4ade80",
        shapeBEnd: "#22c55e",
        shapeCStart: "#dcfce7",
        shapeCMiddle: "#bbf7d0",
        shapeCEnd: "#4ade80",
        shapeDStart: "#bbf7d0",
        shapeDMiddle: "#86efac",
        shapeDEnd: "#4ade80",
        shadowColor1: `rgba(34, 197, 94, ${0.4 * intensity})`,
        shadowColor2: `rgba(22, 163, 74, ${0.5 * intensity})`,
        shadowColor3: `rgba(74, 222, 128, ${0.2 * intensity})`,
        shadowColor4: `rgba(134, 239, 172, ${0.3 * intensity})`,
        // Much gentler speed variation
        speed: 0.4 + level * 0.2,
        hueRotation: 15 + level * 10,
    };
}

function getTalkingPalette(level: number): OrbPalette {
    const base = PALETTES.talking;
    const intensity = 0.5 + level * 0.5;
    return {
        ...base,
        shadowColor1: `rgba(59, 130, 246, ${0.4 * intensity})`,
        shadowColor2: `rgba(37, 99, 235, ${0.5 * intensity})`,
        shadowColor3: `rgba(96, 165, 250, ${0.3 * intensity})`,
        shadowColor4: `rgba(147, 197, 253, ${0.4 * intensity})`,
        speed: 0.4 + level * 0.15,
        hueRotation: 15 + level * 8,
    };
}

function SvgElements({ color1, color2 }: SvgElementsProps) {
    return (
        <>
            <svg
                viewBox="0 0 100 100"
                xmlns="http://www.w3.org/2000/svg"
                className="orb-blob-a"
            >
                <defs>
                    <linearGradient id="blob-a-gradient" x1="0" x2="1" y1="1" y2="0">
                        <stop stopColor={color1} offset="0%" />
                        <stop stopColor={color2} offset="100%" />
                    </linearGradient>
                </defs>
                <path
                    fill="url(#blob-a-gradient)"
                    d="M23.3,-31.9C28.4,-28.4,29.5,-19.2,31.1,-10.9C32.6,-2.6,34.7,4.7,33.5,11.7C32.3,18.6,27.8,25.3,21.6,28.8C15.5,32.3,7.8,32.6,-0.6,33.5C-9,34.4,-18.1,35.8,-24.8,32.5C-31.5,29.2,-35.9,21.2,-36.5,13.3C-37.2,5.4,-34.1,-2.4,-31.6,-10.4C-29.1,-18.3,-27.2,-26.6,-22.1,-30.1C-16.9,-33.6,-8.4,-32.3,0.3,-32.8C9.1,-33.3,18.2,-35.4,23.3,-31.9Z"
                    transform="translate(50 50)"
                />
            </svg>

            <svg
                viewBox="0 0 100 100"
                xmlns="http://www.w3.org/2000/svg"
                className="orb-blob-b"
            >
                <defs>
                    <linearGradient id="blob-b-gradient" x1="0" x2="1" y1="1" y2="0">
                        <stop stopColor={color1} offset="0%" />
                        <stop stopColor={color2} offset="100%" />
                    </linearGradient>
                </defs>
                <path
                    fill="url(#blob-b-gradient)"
                    d="M19.2,-27.1C25.6,-29.4,32.3,-26,33.8,-20.5C35.3,-15,31.7,-7.5,27.8,-2.2C23.9,3,19.8,6,16.8,8.9C13.8,11.8,11.9,14.6,9.3,18.8C6.7,23,3.3,28.6,-0.7,29.8C-4.7,31,-9.4,27.8,-16.7,26.2C-23.9,24.7,-33.6,24.8,-39.3,20.7C-45.1,16.7,-47,8.3,-45.4,0.9C-43.8,-6.5,-38.8,-13,-32.6,-16.3C-26.4,-19.6,-18.9,-19.7,-13.3,-17.9C-7.6,-16,-3.8,-12.2,1.3,-14.4C6.3,-16.6,12.7,-24.8,19.2,-27.1Z"
                    transform="translate(50 50)"
                />
            </svg>

            <svg
                viewBox="0 0 100 100"
                xmlns="http://www.w3.org/2000/svg"
                className="orb-blob-shine"
            >
                <path
                    fill="white"
                    d="M12.3,-22.8C17.4,-18.4,23.8,-17.9,26.8,-14.8C29.8,-11.6,29.5,-5.8,29,-0.3C28.6,5.3,28,10.6,26.9,17.1C25.9,23.7,24.3,31.5,19.7,32.2C15.2,32.8,7.6,26.3,0.6,25.3C-6.4,24.3,-12.9,28.9,-16.8,27.9C-20.8,26.9,-22.3,20.3,-23.8,14.7C-25.3,9.2,-26.9,4.6,-30.1,-1.8C-33.3,-8.3,-38.1,-16.5,-35.4,-20.1C-32.7,-23.6,-22.5,-22.4,-15.3,-25.5C-8.2,-28.6,-4.1,-36,-0.2,-35.6C3.6,-35.2,7.3,-27.1,12.3,-22.8Z"
                    transform="translate(50 50)"
                />
            </svg>

            <svg
                viewBox="0 0 100 100"
                xmlns="http://www.w3.org/2000/svg"
                className="orb-blob-shine orb-shine-b"
            >
                <path
                    fill="white"
                    d="M12.3,-22.8C17.4,-18.4,23.8,-17.9,26.8,-14.8C29.8,-11.6,29.5,-5.8,29,-0.3C28.6,5.3,28,10.6,26.9,17.1C25.9,23.7,24.3,31.5,19.7,32.2C15.2,32.8,7.6,26.3,0.6,25.3C-6.4,24.3,-12.9,28.9,-16.8,27.9C-20.8,26.9,-22.3,20.3,-23.8,14.7C-25.3,9.2,-26.9,4.6,-30.1,-1.8C-33.3,-8.3,-38.1,-16.5,-35.4,-20.1C-32.7,-23.6,-22.5,-22.4,-15.3,-25.5C-8.2,-28.6,-4.1,-36,-0.2,-35.6C3.6,-35.2,7.3,-27.1,12.3,-22.8Z"
                    transform="translate(50 50)"
                />
            </svg>
        </>
    );
}

export function Orb({ state, level = 0, className = "" }: OrbProps) {
    const palette = useMemo(() => {
        switch (state) {
            case "idle":
                return PALETTES.idle;
            case "talking":
                return getTalkingPalette(level);
            case "listening":
                return getListeningPalette(level);
            default:
                return PALETTES.idle;
        }
    }, [state, level]);

    // Fixed base size - use transform scale for breathing effect
    const baseSize = 120;
    const shapeSize = 110;

    // More pronounced scale: talking expands (1.0 to 1.25), listening contracts (1.0 to 0.82)
    let orbScale = 1;
    if (state === "talking") {
        orbScale = 1 + level * 0.25; // Expand outward when talking (up to 25% larger)
    } else if (state === "listening") {
        orbScale = 1 - level * 0.18; // Contract when listening (up to 18% smaller)
    }

    const containerStyle: CSSProperties = {
        "--orb-size": `${baseSize}px`,
        "--shape-size": `${shapeSize}px`,
        "--orb-scale": orbScale,
        "--main-bg-start": palette.mainBgStart,
        "--main-bg-end": palette.mainBgEnd,
        "--shape-a-start": palette.shapeAStart,
        "--shape-a-end": palette.shapeAEnd,
        "--shape-b-start": palette.shapeBStart,
        "--shape-b-middle": palette.shapeBMiddle,
        "--shape-b-end": palette.shapeBEnd,
        "--shape-c-start": palette.shapeCStart,
        "--shape-c-middle": palette.shapeCMiddle,
        "--shape-c-end": palette.shapeCEnd,
        "--shape-d-start": palette.shapeDStart,
        "--shape-d-middle": palette.shapeDMiddle,
        "--shape-d-end": palette.shapeDEnd,
        "--shadow-color-1": palette.shadowColor1,
        "--shadow-color-2": palette.shadowColor2,
        "--shadow-color-3": palette.shadowColor3,
        "--shadow-color-4": palette.shadowColor4,
        "--animation-speed": `${1 / (palette.speed * 0.5)}s`,
        "--hue-rotation": `${palette.hueRotation}deg`,
        "--blob-a-opacity": state !== "idle" ? 0.4 + level * 0.3 : 0.5,
        "--blob-b-opacity": state !== "idle" ? 0.3 + level * 0.2 : 0.4,
    } as CSSProperties;

    return (
        <div className={`orb-container ${className}`} style={containerStyle}>
            <div className="orb-main">
                <div className="orb-glass" />
                <div className="orb-shape-a" />
                <div className="orb-shape-b" />
                <div className="orb-shape-c" />
                <div className="orb-shape-d" />
                <SvgElements color1={palette.mainBgStart} color2={palette.mainBgEnd} />
            </div>
        </div>
    );
}

export default Orb;
