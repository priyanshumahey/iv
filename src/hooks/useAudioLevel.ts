import { useCallback, useEffect, useRef, useState } from "react";

interface AudioLevelHook {
    levelRef: React.MutableRefObject<number>;
    ready: boolean;
    error: string | null;
    start: () => Promise<void>;
    stop: () => void;
}

/**
 * Hook to capture real-time audio levels from the microphone.
 * Returns a ref containing the current normalized audio level (0-1).
 */
export function useAudioLevel(): AudioLevelHook {
    const levelRef = useRef<number>(0);
    const streamRef = useRef<MediaStream | null>(null);
    const analyserRef = useRef<AnalyserNode | null>(null);
    const ctxRef = useRef<AudioContext | null>(null);
    const rafRef = useRef<number>(0);
    const [ready, setReady] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const stop = useCallback(() => {
        cancelAnimationFrame(rafRef.current);
        ctxRef.current?.close();
        streamRef.current?.getTracks().forEach((t) => t.stop());
        ctxRef.current = null;
        streamRef.current = null;
        analyserRef.current = null;
        levelRef.current = 0;
        setReady(false);
    }, []);

    const start = useCallback(async () => {
        stop();
        try {
            const AudioContextClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
            const ctx = new AudioContextClass();
            ctxRef.current = ctx;

            const analyser = ctx.createAnalyser();
            analyser.fftSize = 1024;
            analyserRef.current = analyser;

            const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
            streamRef.current = stream;

            const source = ctx.createMediaStreamSource(stream);
            source.connect(analyser);

            const data = new Uint8Array(analyser.frequencyBinCount);

            const tick = () => {
                analyser.getByteFrequencyData(data);
                const avg = data.reduce((a, b) => a + b, 0) / data.length;
                // Normalize to 0-1 range with gentler mapping
                const norm = Math.min(1, Math.max(0, (avg - 20) / 80));
                // Slower interpolation for smoother animation
                levelRef.current += (norm - levelRef.current) * 0.06;
                rafRef.current = requestAnimationFrame(tick);
            };

            tick();
            setReady(true);
            setError(null);
        } catch (err) {
            const message = err instanceof Error ? err.message : "Unknown error";
            setError(message);
            setReady(false);
        }
    }, [stop]);

    useEffect(() => {
        return () => stop();
    }, [stop]);

    return { levelRef, ready, error, start, stop };
}
