import { useCallback, useEffect, useRef } from "react";

interface TalkingSimulationHook {
    levelRef: React.MutableRefObject<number>;
    start: () => void;
    stop: () => void;
    isActive: boolean;
}

/**
 * Simulates natural speech patterns for the "talking" orb state.
 * Creates organic-looking audio level variations that mimic real speech.
 */
export function useTalkingSimulation(): TalkingSimulationHook {
    const levelRef = useRef<number>(0);
    const rafRef = useRef<number>(0);
    const isActiveRef = useRef<boolean>(false);
    const timeRef = useRef<number>(0);
    const phraseRef = useRef<number>(0);
    const pauseUntilRef = useRef<number>(0);

    const stop = useCallback(() => {
        isActiveRef.current = false;
        cancelAnimationFrame(rafRef.current);
        levelRef.current = 0;
        timeRef.current = 0;
        phraseRef.current = 0;
        pauseUntilRef.current = 0;
    }, []);

    const start = useCallback(() => {
        stop();
        isActiveRef.current = true;
        timeRef.current = performance.now();
        pauseUntilRef.current = 0;

        const tick = () => {
            if (!isActiveRef.current) return;

            const now = performance.now();
            const elapsed = (now - timeRef.current) / 1000;

            // Check if we're in a pause (between phrases)
            if (now < pauseUntilRef.current) {
                // During pause, smoothly decay to a low level
                levelRef.current += (0.15 - levelRef.current) * 0.03;
                rafRef.current = requestAnimationFrame(tick);
                return;
            }

            // Simulate natural speech cadence with gentler waves
            // Use slower frequencies for smoother motion
            const baseFreq = 1.2; // Slow breathing rhythm
            const wordFreq = 2.5; // Gentle word emphasis
            const syllableFreq = 4; // Subtle syllable detail

            // Main speech envelope with phrases (longer, smoother phrases)
            const phraseLength = 3 + Math.sin(phraseRef.current * 0.5) * 1;
            const phraseProgress = elapsed % phraseLength;
            // Smoother envelope using smoothstep-like curve
            const t = phraseProgress / phraseLength;
            const phraseEnvelope = Math.sin(t * Math.PI) * 0.5 + 0.5;

            // Check if phrase just ended - add natural pause
            if (phraseProgress < 0.05 && elapsed > 0.5) {
                phraseRef.current += 1;
                // Random pause between 300ms and 700ms
                const pauseDuration = 300 + Math.random() * 400;
                pauseUntilRef.current = now + pauseDuration;
            }

            // Word-level variation (gentler, less extreme)
            const wordLevel = 0.6 + 0.4 * Math.sin(elapsed * wordFreq);

            // Syllable detail (subtle modulation)
            const syllableLevel = 0.9 + 0.1 * Math.sin(elapsed * syllableFreq);

            // Combine all factors with a stable base
            const base = 0.4 + Math.sin(elapsed * baseFreq) * 0.15;
            const target = base + phraseEnvelope * wordLevel * syllableLevel * 0.45;

            // Clamp to reasonable range and use very slow interpolation
            const clamped = Math.min(0.9, Math.max(0.2, target));
            levelRef.current += (clamped - levelRef.current) * 0.04;

            rafRef.current = requestAnimationFrame(tick);
        };

        tick();
    }, [stop]);

    useEffect(() => {
        return () => stop();
    }, [stop]);

    return {
        levelRef,
        start,
        stop,
        isActive: isActiveRef.current,
    };
}
