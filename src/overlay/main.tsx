import React, { useState, useEffect } from 'react';
import ReactDOM from 'react-dom/client';
import { listen } from '@tauri-apps/api/event';
import { Waveform } from './Waveform';
import './overlay.css';

type OverlayState = 'hidden' | 'recording' | 'transcribing';

function RecordingOverlay() {
    const [state, setState] = useState<OverlayState>('hidden');
    const [audioLevel, setAudioLevel] = useState(0);

    useEffect(() => {
        // Listen for state changes from the backend
        const unlisten = listen<OverlayState>('overlay-state-change', (event) => {
            setState(event.payload);
        });

        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    useEffect(() => {
        // Listen for audio level updates from the backend
        const unlisten = listen<number>('audio-level', (event) => {
            setAudioLevel(event.payload);
        });

        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    if (state === 'hidden') {
        return null;
    }

    const isRecording = state === 'recording';
    const isTranscribing = state === 'transcribing';

    return (
        <div className={`overlay-container ${state}`}>
            <div className="waveform-wrapper">
                <Waveform
                    audioLevel={audioLevel}
                    isActive={isRecording}
                    isProcessing={isTranscribing}
                    barWidth={3}
                    barGap={1}
                    sensitivity={2.5}
                    fadeWidth={14}
                />
            </div>
        </div>
    );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
        <RecordingOverlay />
    </React.StrictMode>
);
