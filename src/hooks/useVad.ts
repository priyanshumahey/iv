import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface UseVadReturn {
    vadEnabled: boolean;
    vadModelDownloaded: boolean;
    vadDownloadProgress: number | null;
    toggleVad: () => Promise<void>;
    ensureVadModel: () => Promise<void>;
}

export function useVad(): UseVadReturn {
    const [vadEnabled, setVadEnabled] = useState(false);
    const [vadModelDownloaded, setVadModelDownloaded] = useState(false);
    const [vadDownloadProgress, setVadDownloadProgress] = useState<number | null>(null);

    // Check VAD state on mount
    useEffect(() => {
        const checkVadState = async () => {
            try {
                const enabled = await invoke<boolean>("is_vad_enabled");
                setVadEnabled(enabled);

                const downloaded = await invoke<boolean>("is_vad_model_downloaded");
                setVadModelDownloaded(downloaded);
            } catch (err) {
                console.error("Failed to check VAD state:", err);
            }
        };

        checkVadState();
    }, []);

    // Toggle VAD
    const toggleVad = useCallback(async () => {
        try {
            const newEnabled = !vadEnabled;
            await invoke("set_vad_enabled", { enabled: newEnabled });
            setVadEnabled(newEnabled);
        } catch (err) {
            console.error("Failed to toggle VAD:", err);
        }
    }, [vadEnabled]);

    // Ensure VAD model is downloaded
    const ensureVadModel = useCallback(async () => {
        try {
            setVadDownloadProgress(0);
            await invoke("ensure_vad_model");
            setVadModelDownloaded(true);
            setVadDownloadProgress(null);
        } catch (err) {
            console.error("Failed to ensure VAD model:", err);
            setVadDownloadProgress(null);
        }
    }, []);

    // Listen for VAD download progress (if backend emits it)
    useEffect(() => {
        const unlistenProgress = listen<{ percentage: number }>("vad-download-progress", (event) => {
            setVadDownloadProgress(event.payload.percentage);
        });

        const unlistenCompleted = listen("vad-download-completed", () => {
            setVadModelDownloaded(true);
            setVadDownloadProgress(null);
        });

        return () => {
            unlistenProgress.then((f) => f());
            unlistenCompleted.then((f) => f());
        };
    }, []);

    // Auto-download VAD model if not present
    useEffect(() => {
        if (!vadModelDownloaded) {
            ensureVadModel();
        }
    }, [vadModelDownloaded, ensureVadModel]);

    return {
        vadEnabled,
        vadModelDownloaded,
        vadDownloadProgress,
        toggleVad,
        ensureVadModel,
    };
}
