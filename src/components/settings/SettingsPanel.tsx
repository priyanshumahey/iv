import { useState } from "react";
import { ChevronDown, ChevronUp, Cloud, Cpu, Download, Trash2, Check, Loader2, Mic, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

// Engine type from backend
type EngineType = "Parakeet" | "Cloud";

// Model info from backend
export interface ModelInfo {
    id: string;
    name: string;
    description: string;
    filename: string;
    url: string | null;
    size_mb: number;
    is_downloaded: boolean;
    is_downloading: boolean;
    partial_size: number;
    is_directory: boolean;
    engine_type: EngineType;
    accuracy_score: number;
    speed_score: number;
}

// Download progress event
export interface DownloadProgress {
    model_id: string;
    downloaded: number;
    total: number;
    percentage: number;
}

interface SettingsPanelProps {
    models: ModelInfo[];
    selectedModel: string;
    downloadProgress: DownloadProgress | null;
    isModelLoading: boolean;
    vadEnabled: boolean;
    vadModelDownloaded: boolean;
    vadDownloadProgress: number | null;
    onSelectModel: (modelId: string) => void;
    onDownloadModel: (modelId: string) => void;
    onDeleteModel: (modelId: string) => void;
    onToggleVad: () => void;
    error: string | null;
}

export function SettingsPanel({
    models,
    selectedModel,
    downloadProgress,
    isModelLoading,
    vadEnabled,
    vadModelDownloaded,
    vadDownloadProgress,
    onSelectModel,
    onDownloadModel,
    onDeleteModel,
    onToggleVad,
    error,
}: SettingsPanelProps) {
    const [isExpanded, setIsExpanded] = useState(false);

    const selectedModelInfo = models.find((m) => m.id === selectedModel);

    return (
        <div className="w-full max-w-md">
            {/* Collapsed view - just shows current model */}
            <button
                onClick={() => setIsExpanded(!isExpanded)}
                className="flex w-full items-center justify-between rounded-lg border border-slate-600/50 bg-slate-800/90 px-4 py-3 text-left backdrop-blur-sm transition-all hover:border-slate-500/60 hover:bg-slate-800"
            >
                <div className="flex items-center gap-3">
                    <Settings className="h-4 w-4 text-slate-400" />
                    <div>
                        <p className="text-sm font-medium">
                            {selectedModelInfo?.name || "Select Model"}
                        </p>
                        <p className="text-xs text-slate-400">
                            {selectedModelInfo?.engine_type === "Cloud" ? "Cloud" : "Local"} •{" "}
                            {vadEnabled ? "VAD On" : "VAD Off"}
                        </p>
                    </div>
                </div>
                {isExpanded ? (
                    <ChevronUp className="h-4 w-4 text-slate-400" />
                ) : (
                    <ChevronDown className="h-4 w-4 text-slate-400" />
                )}
            </button>

            {/* Expanded settings */}
            {isExpanded && (
                <div className="mt-2 space-y-4 rounded-lg border border-slate-600/50 bg-slate-800/95 p-4 backdrop-blur-sm">
                    {/* Error display */}
                    {error && (
                        <div className="rounded-md border border-red-700/40 bg-red-900/30 px-3 py-2 text-xs text-red-200">
                            {error}
                        </div>
                    )}

                    {/* Model Selection */}
                    <div className="space-y-2">
                        <h3 className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-slate-300">
                            <Cpu className="h-3 w-3 text-slate-400" />
                            Transcription Model
                        </h3>
                        <div className="space-y-1">
                            {models.map((model) => (
                                <ModelCard
                                    key={model.id}
                                    model={model}
                                    isSelected={selectedModel === model.id}
                                    downloadProgress={
                                        downloadProgress?.model_id === model.id
                                            ? downloadProgress
                                            : null
                                    }
                                    isModelLoading={isModelLoading && selectedModel === model.id}
                                    onSelect={() => onSelectModel(model.id)}
                                    onDownload={() => onDownloadModel(model.id)}
                                    onDelete={() => onDeleteModel(model.id)}
                                />
                            ))}
                        </div>
                    </div>

                    {/* VAD Toggle */}
                    <div className="space-y-2">
                        <h3 className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-slate-300">
                            <Mic className="h-3 w-3 text-slate-400" />
                            Voice Activity Detection
                        </h3>
                        <button
                            onClick={onToggleVad}
                            disabled={!vadModelDownloaded}
                            className={cn(
                                "flex w-full items-center justify-between rounded-md border px-3 py-2 text-sm transition-all disabled:cursor-not-allowed disabled:opacity-60",
                                vadEnabled
                                    ? "border-emerald-400/40 bg-emerald-500/15 text-emerald-200"
                                    : "border-slate-600/50 bg-slate-700/60 text-slate-200 hover:border-slate-500/60 hover:bg-slate-700/80"
                            )}
                        >
                            <span>Filter silence from audio</span>
                            <span
                                className={cn(
                                    "rounded-full px-2 py-0.5 text-xs font-medium",
                                    vadEnabled ? "bg-emerald-500/20 text-emerald-200" : "bg-slate-600/80 text-slate-200"
                                )}
                            >
                                {vadEnabled ? "On" : "Off"}
                            </span>
                        </button>
                        {vadDownloadProgress !== null && (
                            <div className="text-xs text-slate-400">
                                Downloading VAD model: {vadDownloadProgress.toFixed(0)}%
                            </div>
                        )}
                        {!vadModelDownloaded && vadDownloadProgress === null && (
                            <div className="text-xs text-amber-300">
                                VAD model downloading...
                            </div>
                        )}
                    </div>

                    {/* Instructions */}
                    <div className="border-t border-slate-600/50 pt-3">
                        <p className="text-xs text-slate-400">
                            Press <kbd className="rounded border border-slate-600/50 bg-slate-700/70 px-1.5 py-0.5 text-slate-200">Ctrl+Space</kbd> to record
                        </p>
                    </div>
                </div>
            )}
        </div>
    );
}

interface ModelCardProps {
    model: ModelInfo;
    isSelected: boolean;
    downloadProgress: DownloadProgress | null;
    isModelLoading: boolean;
    onSelect: () => void;
    onDownload: () => void;
    onDelete: () => void;
}

function ModelCard({
    model,
    isSelected,
    downloadProgress,
    isModelLoading,
    onSelect,
    onDownload,
    onDelete,
}: ModelCardProps) {
    const isCloud = model.engine_type === "Cloud";
    const canSelect = isCloud || model.is_downloaded;
    const isDownloading = model.is_downloading || downloadProgress !== null;

    return (
        <div
            className={cn(
                "group relative rounded-md border px-3 py-2 transition-all",
                isSelected
                    ? "border-blue-400/60 bg-blue-500/15 shadow-sm"
                    : "border-slate-600/50 bg-slate-700/70 hover:border-slate-500/60 hover:bg-slate-700/90"
            )}
        >
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                    {isCloud ? (
                        <Cloud className="h-4 w-4 text-blue-400" />
                    ) : (
                        <Cpu className="h-4 w-4 text-emerald-400" />
                    )}
                    <div>
                        <p className="text-sm font-medium text-slate-200">{model.name}</p>
                        <p className="text-xs text-slate-400">
                            {isCloud ? "OpenAI API" : `${model.size_mb} MB`}
                        </p>
                    </div>
                </div>

                <div className="flex items-center gap-1">
                    {/* Download button for local models */}
                    {!isCloud && !model.is_downloaded && !isDownloading && (
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                onDownload();
                            }}
                            className="rounded p-1.5 text-slate-400 transition-colors hover:bg-slate-600/60 hover:text-slate-200"
                            title="Download model"
                        >
                            <Download className="h-4 w-4" />
                        </button>
                    )}

                    {/* Downloading indicator */}
                    {isDownloading && (
                        <div className="flex items-center gap-2">
                            <Loader2 className="h-4 w-4 animate-spin text-blue-400" />
                            {downloadProgress && (
                                <span className="text-xs font-medium text-blue-200">
                                    {downloadProgress.percentage.toFixed(0)}%
                                </span>
                            )}
                        </div>
                    )}

                    {/* Select button */}
                    {canSelect && !isDownloading && (
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                onSelect();
                            }}
                            disabled={isSelected || isModelLoading}
                            className={cn(
                                "rounded p-1.5 transition-colors disabled:cursor-not-allowed disabled:opacity-60",
                                isSelected
                                    ? "text-blue-400"
                                    : "text-slate-400 hover:bg-slate-600/60 hover:text-slate-200"
                            )}
                            title={isSelected ? "Selected" : "Select model"}
                        >
                            {isModelLoading ? (
                                <Loader2 className="h-4 w-4 animate-spin" />
                            ) : isSelected ? (
                                <Check className="h-4 w-4" />
                            ) : (
                                <Check className="h-4 w-4 opacity-0 group-hover:opacity-60" />
                            )}
                        </button>
                    )}

                    {/* Delete button for downloaded local models */}
                    {!isCloud && model.is_downloaded && !isSelected && (
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                onDelete();
                            }}
                            className="rounded p-1.5 text-slate-400 opacity-0 transition-all hover:bg-red-500/20 hover:text-red-200 group-hover:opacity-100"
                            title="Delete model"
                        >
                            <Trash2 className="h-4 w-4" />
                        </button>
                    )}
                </div>
            </div>

            {/* Download progress bar */}
            {downloadProgress && (
                <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-slate-600/70">
                    <div
                        className="h-full rounded-full bg-gradient-to-r from-blue-500 to-blue-400 ring-1 ring-blue-400/20 transition-all"
                        style={{ width: `${downloadProgress.percentage}%` }}
                    />
                </div>
            )}
        </div>
    );
}

export default SettingsPanel;