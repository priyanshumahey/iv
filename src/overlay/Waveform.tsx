import { useEffect, useRef } from 'react';

type WaveformProps = {
    audioLevel?: number;
    isActive: boolean;
    isProcessing: boolean;
    barWidth?: number;
    barGap?: number;
    barRadius?: number;
    barHeight?: number;
    sensitivity?: number;
    fadeEdges?: boolean;
    fadeWidth?: number;
};

export function Waveform({
    audioLevel = 0,
    isActive,
    isProcessing,
    barWidth = 3,
    barGap = 1,
    barRadius = 1.5,
    barHeight: baseBarHeight = 4,
    sensitivity = 2.5,
    fadeEdges = true,
    fadeWidth = 12,
}: WaveformProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const animationRef = useRef<number>(0);

    // Audio level history - this creates the actual waveform visualization
    const levelHistoryRef = useRef<number[]>([]);
    const maxHistorySize = 100;

    // For smooth bar values
    const displayBarsRef = useRef<number[]>([]);

    // For scrolling mode (transcribing)
    const processingTimeRef = useRef(0);
    const transitionProgressRef = useRef(0);
    const lastActiveDataRef = useRef<number[]>([]);

    // Gradient cache
    const gradientCacheRef = useRef<CanvasGradient | null>(null);
    const lastWidthRef = useRef(0);

    // Handle canvas resizing
    useEffect(() => {
        const canvas = canvasRef.current;
        const container = containerRef.current;
        if (!canvas || !container) return;

        const resizeObserver = new ResizeObserver(() => {
            const rect = container.getBoundingClientRect();
            const dpr = window.devicePixelRatio || 1;

            canvas.width = rect.width * dpr;
            canvas.height = rect.height * dpr;
            canvas.style.width = `${rect.width}px`;
            canvas.style.height = `${rect.height}px`;

            const ctx = canvas.getContext('2d');
            if (ctx) {
                ctx.scale(dpr, dpr);
            }

            gradientCacheRef.current = null;
            lastWidthRef.current = rect.width;
        });

        resizeObserver.observe(container);
        return () => resizeObserver.disconnect();
    }, []);

    // Main animation loop
    useEffect(() => {
        const canvas = canvasRef.current;
        const container = containerRef.current;
        if (!canvas || !container) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        let lastAudioLevel = 0;

        const animate = () => {
            const rect = container.getBoundingClientRect();
            const step = barWidth + barGap;
            const barCount = Math.floor(rect.width / step);
            const centerY = rect.height / 2;
            const halfCount = Math.floor(barCount / 2);

            ctx.clearRect(0, 0, rect.width, rect.height);

            if (isActive) {
                // RECORDING MODE - Show actual audio waveform using level history
                transitionProgressRef.current = 0;

                // Add current audio level to history
                if (audioLevel !== lastAudioLevel || levelHistoryRef.current.length === 0) {
                    levelHistoryRef.current.push(audioLevel);
                    if (levelHistoryRef.current.length > maxHistorySize) {
                        levelHistoryRef.current.shift();
                    }
                    lastAudioLevel = audioLevel;
                }

                // Initialize display bars if needed
                if (displayBarsRef.current.length !== barCount) {
                    displayBarsRef.current = new Array(barCount).fill(0.03);
                }

                const history = levelHistoryRef.current;
                const historyLen = history.length;

                // Map history to bars - center shows recent, edges show older
                // This creates a symmetric "spreading" waveform effect
                for (let i = 0; i < barCount; i++) {
                    // Calculate distance from center (0 = center, 1 = edge)
                    const distFromCenter = Math.abs(i - halfCount) / halfCount;

                    // Map bar position to history index
                    // Center = most recent, edges = older data
                    const historyOffset = Math.floor(distFromCenter * Math.min(historyLen, halfCount * 0.8));
                    const historyIndex = Math.max(0, historyLen - 1 - historyOffset);

                    // Get the audio level for this time slice
                    let level = historyLen > 0 ? (history[historyIndex] || 0) : 0;

                    // Apply power curve for better dynamic range
                    // This makes quiet sounds very small and loud sounds fill the space
                    level = Math.pow(level, 0.5) * sensitivity;

                    // Clamp
                    level = Math.max(0.02, Math.min(1, level));

                    // Smooth transition - fast attack, slower decay
                    const current = displayBarsRef.current[i] || 0.02;
                    const speed = level > current ? 0.6 : 0.12;
                    displayBarsRef.current[i] = current + (level - current) * speed;
                }

                // Save for transition
                lastActiveDataRef.current = [...displayBarsRef.current];

                // Draw bars
                for (let i = 0; i < barCount; i++) {
                    const value = displayBarsRef.current[i];
                    const x = i * step;
                    const height = Math.max(baseBarHeight, value * rect.height * 0.9);
                    const y = centerY - height / 2;

                    const alpha = 0.35 + value * 0.65;
                    ctx.fillStyle = `rgba(255, 68, 68, ${alpha})`;
                    ctx.beginPath();
                    ctx.roundRect(x, y, barWidth, height, barRadius);
                    ctx.fill();
                }

            } else if (isProcessing) {
                // PROCESSING MODE - Flowing wave animation
                processingTimeRef.current += 0.04;
                transitionProgressRef.current = Math.min(1, transitionProgressRef.current + 0.025);

                // Initialize display bars if needed
                if (displayBarsRef.current.length !== barCount) {
                    displayBarsRef.current = new Array(barCount).fill(0.2);
                }

                const time = processingTimeRef.current;

                for (let i = 0; i < barCount; i++) {
                    // Create flowing wave that moves across bars
                    const phase = (i / barCount) * Math.PI * 4 - time * 2.5;
                    const wave1 = Math.sin(phase) * 0.35;
                    const wave2 = Math.sin(phase * 0.6 + time * 0.7) * 0.2;
                    const wave3 = Math.cos(phase * 0.3 - time * 0.4) * 0.12;

                    let targetValue = 0.3 + wave1 + wave2 + wave3;
                    targetValue = Math.max(0.08, Math.min(0.85, targetValue));

                    // Blend with last active data during transition
                    if (lastActiveDataRef.current.length > 0 && transitionProgressRef.current < 1) {
                        const lastValue = lastActiveDataRef.current[i] || 0.2;
                        targetValue = lastValue * (1 - transitionProgressRef.current) +
                            targetValue * transitionProgressRef.current;
                    }

                    // Smooth transition
                    const current = displayBarsRef.current[i] || 0.2;
                    displayBarsRef.current[i] = current + (targetValue - current) * 0.12;
                }

                // Draw bars
                for (let i = 0; i < barCount; i++) {
                    const value = displayBarsRef.current[i];
                    const x = i * step;
                    const height = Math.max(baseBarHeight, value * rect.height * 0.9);
                    const y = centerY - height / 2;

                    const alpha = 0.35 + value * 0.65;
                    ctx.fillStyle = `rgba(68, 136, 255, ${alpha})`;
                    ctx.beginPath();
                    ctx.roundRect(x, y, barWidth, height, barRadius);
                    ctx.fill();
                }

            } else {
                // IDLE - Fade out
                if (displayBarsRef.current.length > 0) {
                    let allFaded = true;

                    for (let i = 0; i < displayBarsRef.current.length; i++) {
                        displayBarsRef.current[i] *= 0.88;
                        if (displayBarsRef.current[i] > 0.015) allFaded = false;
                    }

                    if (!allFaded) {
                        for (let i = 0; i < displayBarsRef.current.length; i++) {
                            const value = displayBarsRef.current[i];
                            const x = i * step;
                            const height = Math.max(baseBarHeight * 0.5, value * rect.height * 0.9);
                            const y = centerY - height / 2;

                            ctx.fillStyle = `rgba(255, 255, 255, ${0.15 + value * 0.5})`;
                            ctx.beginPath();
                            ctx.roundRect(x, y, barWidth, height, barRadius);
                            ctx.fill();
                        }
                    } else {
                        displayBarsRef.current = [];
                    }
                }

                // Reset state
                levelHistoryRef.current = [];
                processingTimeRef.current = 0;
            }

            // Apply edge fading
            if (fadeEdges && fadeWidth > 0 && rect.width > 0) {
                if (!gradientCacheRef.current || lastWidthRef.current !== rect.width) {
                    const gradient = ctx.createLinearGradient(0, 0, rect.width, 0);
                    const fadePercent = Math.min(0.2, fadeWidth / rect.width);

                    gradient.addColorStop(0, 'rgba(255,255,255,1)');
                    gradient.addColorStop(fadePercent, 'rgba(255,255,255,0)');
                    gradient.addColorStop(1 - fadePercent, 'rgba(255,255,255,0)');
                    gradient.addColorStop(1, 'rgba(255,255,255,1)');

                    gradientCacheRef.current = gradient;
                    lastWidthRef.current = rect.width;
                }

                ctx.globalCompositeOperation = 'destination-out';
                ctx.fillStyle = gradientCacheRef.current;
                ctx.fillRect(0, 0, rect.width, rect.height);
                ctx.globalCompositeOperation = 'source-over';
            }

            ctx.globalAlpha = 1;
            animationRef.current = requestAnimationFrame(animate);
        };

        animationRef.current = requestAnimationFrame(animate);

        return () => {
            if (animationRef.current) {
                cancelAnimationFrame(animationRef.current);
            }
        };
    }, [audioLevel, isActive, isProcessing, barWidth, barGap, barRadius, baseBarHeight, sensitivity, fadeEdges, fadeWidth]);

    return (
        <div
            ref={containerRef}
            className="waveform-container"
            aria-label={isActive ? 'Live audio waveform' : isProcessing ? 'Processing audio' : 'Audio waveform idle'}
            role="img"
        >
            <canvas
                ref={canvasRef}
                className="waveform-canvas"
                aria-hidden="true"
            />
        </div>
    );
}
