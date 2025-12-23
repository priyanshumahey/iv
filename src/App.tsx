import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast, Toaster } from "sonner";
import { Copy, Keyboard } from "lucide-react";
import "./App.css";
import { Orb, type OrbState } from "./components/orb";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { useAudioLevel } from "./hooks/useAudioLevel";
import { useTalkingSimulation } from "./hooks/useTalkingSimulation";
import { useModels } from "./hooks/useModels";
import { useVad } from "./hooks/useVad";

// Sound effect paths - place your audio files in the public folder
const SOUND_VOICE_ON = "/sounds/voice-on.mp3";
const SOUND_VOICE_OFF = "/sounds/voice-off.mp3";

export default function App() {
  const [state, setState] = useState<OrbState>("idle");
  const [level, setLevel] = useState(0);
  const [lastTranscription, setLastTranscription] = useState<string | null>(null);

  // Model management
  const {
    models,
    selectedModel,
    downloadProgress,
    isModelLoading,
    error: modelError,
    selectModel,
    downloadModel,
    deleteModel,
  } = useModels();

  // VAD management
  const {
    vadEnabled,
    vadModelDownloaded,
    vadDownloadProgress,
    toggleVad,
  } = useVad();

  // Web Audio API for real-time mic levels during recording
  const {
    levelRef: audioLevelRef,
    start: startAudio,
    stop: stopAudio,
  } = useAudioLevel();

  // Audio refs for sound effects
  const voiceOnRef = useRef<HTMLAudioElement | null>(null);
  const voiceOffRef = useRef<HTMLAudioElement | null>(null);

  // Initialize audio elements
  useEffect(() => {
    voiceOnRef.current = new Audio(SOUND_VOICE_ON);
    voiceOffRef.current = new Audio(SOUND_VOICE_OFF);

    // Preload
    voiceOnRef.current.load();
    voiceOffRef.current.load();

    return () => {
      voiceOnRef.current = null;
      voiceOffRef.current = null;
    };
  }, []);

  // Listen for Tauri recording events (from global shortcut)
  useEffect(() => {
    const unlistenStarted = listen("recording-started", () => {
      setState("listening");
      startAudio(); // Start capturing mic levels for orb animation
      voiceOnRef.current?.play().catch(console.error);
    });

    const unlistenStopped = listen("recording-stopped", () => {
      setState("talking");
      stopAudio(); // Stop mic capture when recording ends
      voiceOffRef.current?.play().catch(console.error);
    });

    const unlistenCompleted = listen<{ text: string }>("transcription-completed", (event) => {
      setState("idle");
      if (event.payload?.text) {
        setLastTranscription(event.payload.text);
        toast.success("Transcription complete", {
          description: event.payload.text.substring(0, 100) + (event.payload.text.length > 100 ? "..." : ""),
        });
      }
    });

    const unlistenError = listen<{ error: string }>("transcription-error", (event) => {
      setState("idle");
      toast.error("Transcription failed", {
        description: event.payload?.error || "Unknown error occurred",
      });
    });

    return () => {
      unlistenStarted.then((f) => f());
      unlistenStopped.then((f) => f());
      unlistenCompleted.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, [startAudio, stopAudio]);

  // Simulated talking animation (for talking mode)
  const {
    levelRef: talkingLevelRef,
    start: startTalking,
    stop: stopTalking,
  } = useTalkingSimulation();

  // Smoothly update level state using requestAnimationFrame
  useEffect(() => {
    let raf = 0;

    const update = () => {
      // Determine target level based on state
      let target = 0;
      if (state === "listening") {
        // Use Web Audio API levels during recording
        target = audioLevelRef.current;
      } else if (state === "talking") {
        target = talkingLevelRef.current;
      }

      // Smooth interpolation with different rates for rise/fall
      // Rise faster, fall slower for more organic feel
      const currentLevel = level;
      const diff = target - currentLevel;
      const rate = diff > 0 ? 0.12 : 0.06; // Rise faster, fall slower

      const newLevel = currentLevel + diff * rate;
      setLevel(newLevel);

      raf = requestAnimationFrame(update);
    };

    update();
    return () => cancelAnimationFrame(raf);
  }, [state, level, audioLevelRef, talkingLevelRef]);

  // Start/stop talking simulation based on state
  useEffect(() => {
    if (state === "talking") {
      startTalking();
    } else {
      stopTalking();
    }
  }, [state, startTalking, stopTalking]);

  const copyTranscription = useCallback(() => {
    if (lastTranscription) {
      navigator.clipboard.writeText(lastTranscription);
      toast.success("Copied to clipboard");
    }
  }, [lastTranscription]);

  const getStateLabel = () => {
    switch (state) {
      case "idle":
        return "Ready";
      case "talking":
        return "Transcribing...";
      case "listening":
        return "Listening...";
    }
  };

  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-gradient-to-br from-slate-900 to-slate-800 p-6">
      <Toaster theme="dark" position="top-center" richColors />

      <div className="flex flex-col items-center gap-8 w-full max-w-md">
        {/* Orb container */}
        <div className="relative">
          <div className="flex h-40 w-40 items-center justify-center rounded-full border-2 border-white/10 bg-white/5 shadow-2xl backdrop-blur-sm">
            <Orb state={state} level={level} />
          </div>

          {/* State indicator */}
          <div className="absolute -bottom-2 left-1/2 -translate-x-1/2">
            <span className={`
              inline-block rounded-full px-3 py-1 text-xs font-medium uppercase tracking-wider
              ${state === "idle" ? "bg-gray-500/20 text-gray-300" : ""}
              ${state === "listening" ? "bg-green-500/20 text-green-400" : ""}
              ${state === "talking" ? "bg-blue-500/20 text-blue-400" : ""}
            `}>
              {getStateLabel()}
            </span>
          </div>
        </div>

        {/* Last transcription result */}
        {lastTranscription && (
          <div className="w-full rounded-lg border border-white/20 bg-slate-800/80 p-4 backdrop-blur-sm">
            <div className="flex items-start justify-between gap-2">
              <p className="flex-1 text-sm text-slate-200 line-clamp-3">
                {lastTranscription}
              </p>
              <button
                onClick={copyTranscription}
                className="shrink-0 rounded p-1.5 text-slate-300 transition-colors hover:bg-slate-700 hover:text-white"
                title="Copy to clipboard"
              >
                <Copy className="h-4 w-4" />
              </button>
            </div>
          </div>
        )}

        {/* Keyboard shortcut hint */}
        <div className="flex items-center gap-2 text-slate-400">
          <Keyboard className="h-4 w-4" />
          <span className="text-xs">
            Press <kbd className="rounded bg-slate-700 px-1.5 py-0.5 text-slate-300 border border-slate-600">Ctrl+Space</kbd> to record
          </span>
        </div>

        {/* Settings Panel */}
        <SettingsPanel
          models={models}
          selectedModel={selectedModel}
          downloadProgress={downloadProgress}
          isModelLoading={isModelLoading}
          vadEnabled={vadEnabled}
          vadModelDownloaded={vadModelDownloaded}
          vadDownloadProgress={vadDownloadProgress}
          onSelectModel={selectModel}
          onDownloadModel={downloadModel}
          onDeleteModel={deleteModel}
          onToggleVad={toggleVad}
          error={modelError}
        />
      </div>
    </div>
  );
}
