import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import { Orb, type OrbState } from "./components/orb";
import { useAudioLevel } from "./hooks/useAudioLevel";
import { useTalkingSimulation } from "./hooks/useTalkingSimulation";

// Sound effect paths - place your audio files in the public folder
const SOUND_VOICE_ON = "/sounds/voice-on.mp3";
const SOUND_VOICE_OFF = "/sounds/voice-off.mp3";

export default function App() {
  const [state, setState] = useState<OrbState>("idle");
  const [level, setLevel] = useState(0);

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

  const playSound = useCallback((sound: "on" | "off") => {
    const audio = sound === "on" ? voiceOnRef.current : voiceOffRef.current;
    if (audio) {
      audio.currentTime = 0;
      audio.play().catch(() => {
        // Ignore autoplay errors
      });
    }
  }, []);

  // Listen for Tauri recording events (from global shortcut)
  useEffect(() => {
    const unlistenStarted = listen("recording-started", () => {
      playSound("on");
      // Also update orb state to listening when recording via shortcut
      setState("listening");
    });

    const unlistenStopped = listen("recording-stopped", () => {
      playSound("off");
      // Switch to talking state while transcribing
      setState("talking");
    });

    const unlistenCompleted = listen("transcription-completed", () => {
      // Return to idle when transcription is done
      setState("idle");
    });

    const unlistenError = listen("transcription-error", () => {
      // Return to idle on error
      setState("idle");
    });

    return () => {
      unlistenStarted.then((f) => f());
      unlistenStopped.then((f) => f());
      unlistenCompleted.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, [playSound]);

  // Audio level from microphone (for listening mode)
  const {
    levelRef: audioLevelRef,
    ready: audioReady,
    error: audioError,
    start: startAudio,
    stop: stopAudio,
  } = useAudioLevel();

  // Simulated talking animation (for talking mode)
  const {
    levelRef: talkingLevelRef,
    start: startTalking,
    stop: stopTalking,
  } = useTalkingSimulation();

  // Smoothly update level state from the appropriate ref
  useEffect(() => {
    let raf = 0;
    const update = () => {
      const targetRef = state === "talking" ? talkingLevelRef : audioLevelRef;
      const targetLevel = state === "idle" ? 0 : targetRef.current;
      // Use slower interpolation for smoother, less jerky animations
      setLevel((prev) => prev + (targetLevel - prev) * 0.08);
      raf = requestAnimationFrame(update);
    };
    update();
    return () => cancelAnimationFrame(raf);
  }, [state, audioLevelRef, talkingLevelRef]);

  const handleStateChange = useCallback(
    async (newState: OrbState) => {
      // Play sounds immediately for responsiveness
      if (state === "listening" && newState !== "listening") {
        playSound("off");
      }
      if (newState === "listening" && state !== "listening") {
        playSound("on");
      }

      // Stop all active effects first
      if (state === "listening" && audioReady) {
        stopAudio();
      }
      if (state === "talking") {
        stopTalking();
      }

      // Start new effects based on state
      if (newState === "listening") {
        await startAudio();
      } else if (newState === "talking") {
        startTalking();
      }

      setState(newState);
    },
    [state, audioReady, stopAudio, stopTalking, startAudio, startTalking, playSound]
  );

  // Toggle handler for listening button - toggles between listening and idle
  const handleListeningToggle = useCallback(async () => {
    if (state === "listening") {
      // Currently listening, turn off
      await handleStateChange("idle");
    } else {
      // Not listening, turn on
      await handleStateChange("listening");
    }
  }, [state, handleStateChange]);

  const getStateLabel = () => {
    switch (state) {
      case "idle":
        return "Idle";
      case "talking":
        return "Speaking...";
      case "listening":
        return "Listening...";
    }
  };

  const getHelpText = () => {
    switch (state) {
      case "listening":
        return "Speak into your microphone to see the orb react to your voice";
      case "talking":
        return "The orb is simulating speech patterns";
      default:
        return "Select a mode to see the orb animate";
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gradient-to-br from-slate-900 to-slate-800">
      <div className="flex flex-col items-center gap-8">
        <div className="flex h-36 w-36 items-center justify-center rounded-full border-2 border-border/60 bg-white shadow-lg">
          <Orb state={state} level={level} />
        </div>

        <div className="text-center">
          <p className="mb-4 text-sm font-medium uppercase tracking-wider text-white">
            {getStateLabel()}
          </p>

          {audioError && (
            <p className="mb-4 text-xs text-red-400">
              Microphone error: {audioError}
            </p>
          )}
        </div>

        <div className="flex gap-3">
          <button
            onClick={() => handleStateChange("idle")}
            className={`rounded-lg px-4 py-2 font-medium transition-all ${state === "idle"
              ? "bg-gray-500 text-white"
              : "bg-gray-700 text-gray-300 hover:bg-gray-600"
              }`}
          >
            Idle
          </button>
          <button
            onClick={() => handleStateChange("talking")}
            className={`rounded-lg px-4 py-2 font-medium transition-all ${state === "talking"
              ? "bg-blue-500 text-white"
              : "bg-blue-900 text-blue-300 hover:bg-blue-800"
              }`}
          >
            Talking
          </button>
          <button
            onClick={handleListeningToggle}
            className={`rounded-lg px-4 py-2 font-medium transition-all ${state === "listening"
              ? "bg-green-500 text-white"
              : "bg-green-900 text-green-300 hover:bg-green-800"
              }`}
          >
            {state === "listening" ? "Stop Listening" : "Listening"}
          </button>
        </div>

        <p className="max-w-md text-center text-xs text-gray-400">
          {getHelpText()}
        </p>
      </div>
    </div>
  );
}