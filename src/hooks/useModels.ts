import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ModelInfo, DownloadProgress } from "../components/settings/SettingsPanel";

interface UseModelsReturn {
    models: ModelInfo[];
    selectedModel: string;
    downloadProgress: DownloadProgress | null;
    isModelLoading: boolean;
    error: string | null;
    refreshModels: () => Promise<void>;
    selectModel: (modelId: string) => Promise<void>;
    downloadModel: (modelId: string) => Promise<void>;
    deleteModel: (modelId: string) => Promise<void>;
}

export function useModels(): UseModelsReturn {
    const [models, setModels] = useState<ModelInfo[]>([]);
    const [selectedModel, setSelectedModel] = useState<string>("");
    const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
    const [isModelLoading, setIsModelLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Fetch available models
    const refreshModels = useCallback(async () => {
        try {
            const availableModels = await invoke<ModelInfo[]>("get_available_models");
            setModels(availableModels);
        } catch (err) {
            console.error("Failed to get models:", err);
            setError(`Failed to load models: ${err}`);
        }
    }, []);

    // Get selected model
    const fetchSelectedModel = useCallback(async () => {
        try {
            const model = await invoke<string>("get_selected_model");
            setSelectedModel(model);
        } catch (err) {
            console.error("Failed to get selected model:", err);
        }
    }, []);

    // Select a model
    const selectModel = useCallback(async (modelId: string) => {
        try {
            setError(null);
            setIsModelLoading(true);
            await invoke("set_selected_model", { modelId });
            setSelectedModel(modelId);
        } catch (err) {
            console.error("Failed to select model:", err);
            setError(`Failed to select model: ${err}`);
        } finally {
            setIsModelLoading(false);
        }
    }, []);

    // Download a model
    const downloadModel = useCallback(async (modelId: string) => {
        try {
            setError(null);
            await invoke("download_model", { modelId });
            // Refresh models after download completes
            await refreshModels();
        } catch (err) {
            console.error("Failed to download model:", err);
            setError(`Failed to download model: ${err}`);
            // Still refresh to update downloading state
            await refreshModels();
        }
    }, [refreshModels]);

    // Delete a model
    const deleteModel = useCallback(async (modelId: string) => {
        try {
            setError(null);
            await invoke("delete_model", { modelId });
            await refreshModels();
        } catch (err) {
            console.error("Failed to delete model:", err);
            setError(`Failed to delete model: ${err}`);
        }
    }, [refreshModels]);

    // Initialize on mount
    useEffect(() => {
        refreshModels();
        fetchSelectedModel();
    }, [refreshModels, fetchSelectedModel]);

    // Listen for download progress events
    useEffect(() => {
        const unlistenProgress = listen<DownloadProgress>("model-download-progress", (event) => {
            setDownloadProgress(event.payload);
        });

        const unlistenStarted = listen<{ model_id: string }>("model-download-started", async (event) => {
            console.log("Download started:", event.payload.model_id);
            setDownloadProgress({
                model_id: event.payload.model_id,
                downloaded: 0,
                total: 0,
                percentage: 0,
            });
            await refreshModels();
        });

        const unlistenCompleted = listen<{ model_id: string }>("model-download-completed", async (event) => {
            console.log("Download completed:", event.payload.model_id);
            setDownloadProgress(null);
            await refreshModels();
        });

        const unlistenFailed = listen<{ model_id: string; error: string }>("model-download-failed", async (event) => {
            console.error("Download failed:", event.payload);
            setDownloadProgress(null);
            setError(`Download failed: ${event.payload.error}`);
            await refreshModels();
        });

        // Listen for model loading events
        const unlistenLoading = listen<{ model_id: string }>("model-loading", () => {
            setIsModelLoading(true);
        });

        const unlistenLoaded = listen<{ model_id: string }>("model-loaded", () => {
            setIsModelLoading(false);
        });

        return () => {
            unlistenProgress.then((f) => f());
            unlistenStarted.then((f) => f());
            unlistenCompleted.then((f) => f());
            unlistenFailed.then((f) => f());
            unlistenLoading.then((f) => f());
            unlistenLoaded.then((f) => f());
        };
    }, [refreshModels]);

    return {
        models,
        selectedModel,
        downloadProgress,
        isModelLoading,
        error,
        refreshModels,
        selectModel,
        downloadModel,
        deleteModel,
    };
}
