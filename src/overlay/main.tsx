import React, { useState, useEffect } from 'react';
import ReactDOM from 'react-dom/client';
import { listen } from '@tauri-apps/api/event';
import './overlay.css';

type OverlayState = 'hidden' | 'recording' | 'transcribing';

function RecordingOverlay() {
    const [state, setState] = useState<OverlayState>('hidden');

    useEffect(() => {
        // Listen for state changes from the backend
        const unlisten = listen<OverlayState>('overlay-state-change', (event) => {
            setState(event.payload);
        });

        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    if (state === 'hidden') {
        return null;
    }

    const statusText = state === 'recording' ? 'Recording...' : 'Transcribing...';

    return (
        <div className={`overlay-container ${state}`}>
            <div className={`indicator ${state}`} />
            <span className="status-text">{statusText}</span>
        </div>
    );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
        <RecordingOverlay />
    </React.StrictMode>
);
