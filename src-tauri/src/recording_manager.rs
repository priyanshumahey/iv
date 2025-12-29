//! Recording Manager - Orchestrates audio recording and transcription

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::audio::AudioRecorder;
use crate::cloud_transcribe::CloudTranscriber;
use crate::local_transcribe::LocalTranscriber;
use crate::models::{EngineType, ModelManager};
use crate::shortcut::events;
use crate::vad::{ensure_vad_model, SileroVad, SmoothedVad, VadFrame, VAD_FRAME_SAMPLES};

#[derive(Clone, Debug, PartialEq)]
pub enum ManagerState {
    Idle,
    Recording,
    Transcribing,
}

pub struct RecordingManager {
    state: Mutex<ManagerState>,
    recorder: Mutex<Option<AudioRecorder>>,
    cloud_transcriber: CloudTranscriber,
    local_transcriber: LocalTranscriber,
    model_manager: Arc<ModelManager>,
    selected_model: Mutex<String>,
    app_handle: AppHandle,
    vad_enabled: Mutex<bool>,
    vad_model_path: Mutex<Option<PathBuf>>,
}

impl RecordingManager {
    pub fn new(
        app_handle: &AppHandle,
        model_manager: Arc<ModelManager>,
    ) -> Result<Self, anyhow::Error> {
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        if api_key.is_none() {
            log::warn!("OPENAI_API_KEY not set. Cloud transcription will fail without it.");
        }

        Ok(Self {
            state: Mutex::new(ManagerState::Idle),
            recorder: Mutex::new(None),
            cloud_transcriber: CloudTranscriber::new(api_key),
            local_transcriber: LocalTranscriber::new(),
            model_manager,
            selected_model: Mutex::new("cloud".to_string()), // Default to cloud
            app_handle: app_handle.clone(),
            vad_enabled: Mutex::new(true),
            vad_model_path: Mutex::new(None),
        })
    }

    /// Get the current state
    pub fn get_state(&self) -> ManagerState {
        self.state.lock().unwrap().clone()
    }

    /// Get the currently selected model ID
    pub fn get_selected_model(&self) -> String {
        self.selected_model.lock().unwrap().clone()
    }

    /// Check if VAD is enabled
    pub fn is_vad_enabled(&self) -> bool {
        *self.vad_enabled.lock().unwrap()
    }

    /// Enable or disable VAD
    pub fn set_vad_enabled(&self, enabled: bool) {
        *self.vad_enabled.lock().unwrap() = enabled;
        log::info!("VAD enabled set to {}", enabled);
    }

    /// Ensure VAD model is downloaded
    pub async fn ensure_vad_model(&self) -> Result<PathBuf, anyhow::Error> {
        let path = ensure_vad_model(&self.app_handle).await?;
        *self.vad_model_path.lock().unwrap() = Some(path.clone());
        Ok(path)
    }

    /// Set the selected model for transcription
    pub fn set_selected_model(&self, model_id: &str) -> Result<(), anyhow::Error> {
        // Validate model exists
        let model_info = self
            .model_manager
            .get_model_info(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        // If it's a local model, check if it's downloaded
        if model_info.engine_type != EngineType::Cloud && !model_info.is_downloaded {
            return Err(anyhow::anyhow!(
                "Model '{}' is not downloaded. Please download it first.",
                model_id
            ));
        }

        // If switching to a local model, load it
        if model_info.engine_type != EngineType::Cloud {
            let model_path = self.model_manager.get_model_path(model_id)?;

            // Check if already loaded
            if self.local_transcriber.current_model().as_deref() != Some(model_id) {
                log::info!("Loading model '{}'...", model_id);

                // Emit loading event
                let _ = self
                    .app_handle
                    .emit("model-loading", serde_json::json!({ "model_id": model_id }));

                self.local_transcriber
                    .load_model(&model_info, &model_path)?;

                // Emit loaded event
                let _ = self
                    .app_handle
                    .emit("model-loaded", serde_json::json!({ "model_id": model_id }));
            }
        } else {
            // Unload local model if switching to cloud
            if self.local_transcriber.is_loaded() {
                self.local_transcriber.unload_model();
            }
        }

        // Update selection
        {
            let mut selected = self.selected_model.lock().unwrap();
            *selected = model_id.to_string();
        }

        log::info!("Selected model: {}", model_id);
        Ok(())
    }

    /// Start recording audio
    pub fn start_recording(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();

        if *state != ManagerState::Idle {
            let current_state = state.clone();
            drop(state); // Release lock before returning
            return Err(anyhow::anyhow!(
                "Cannot start recording: currently {:?}. Please wait for the current operation to complete.",
                current_state
            ));
        }

        // Create and open the recorder
        let mut recorder = AudioRecorder::new()?;
        recorder.open(None)?;
        recorder.start()?;

        *self.recorder.lock().unwrap() = Some(recorder);
        *state = ManagerState::Recording;

        let _ = self.app_handle.emit(events::RECORDING_STARTED, ());

        log::info!("Recording started.");
        Ok(())
    }

    /// Stop recording and transcribe
    pub async fn stop_and_transcribe(&self) -> Result<String, anyhow::Error> {
        let (samples, sample_rate) = {
            let mut state = self.state.lock().unwrap();
            let mut recorder_guard = self.recorder.lock().unwrap();

            if *state != ManagerState::Recording {
                return Err(anyhow::anyhow!(
                    "Cannot stop: not currently recording (state: {:?})",
                    *state
                ));
            }

            let recorder = recorder_guard
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("Recorder not initialized"))?;

            let samples = recorder.stop()?;
            let sample_rate = recorder.sample_rate();

            recorder.close()?;
            *recorder_guard = None;
            *state = ManagerState::Transcribing;

            let _ = self.app_handle.emit(events::RECORDING_STOPPED, ());

            (samples, sample_rate)
        };

        if samples.is_empty() {
            let mut state = self.state.lock().unwrap();
            *state = ManagerState::Idle;
            return Err(anyhow::anyhow!("No audio recorded"));
        }

        log::info!(
            "Captured {} samples at {} Hz ({:.2}s of audio)",
            samples.len(),
            sample_rate,
            samples.len() as f32 / sample_rate as f32
        );

        // Get selected model
        let model_id = self.get_selected_model();
        let model_info = self
            .model_manager
            .get_model_info(&model_id)
            .ok_or_else(|| anyhow::anyhow!("Selected model not found"))?;

        // Resample to 16kHz if needed (required for all models and VAD)
        let samples_16k = if sample_rate != 16000 {
            let resampled = resample_to_16k(&samples, sample_rate);
            log::info!(
                "Resampled audio: {} Hz → 16000 Hz ({} → {} samples)",
                sample_rate,
                samples.len(),
                resampled.len()
            );
            resampled
        } else {
            samples
        };

        // Apply VAD if enabled
        let samples_filtered = if self.is_vad_enabled() {
            let vad_path = self.vad_model_path.lock().unwrap().clone();
            if let Some(path) = vad_path {
                match self.filter_with_vad(&samples_16k, &path) {
                    Ok(filtered) => {
                        let original_duration = samples_16k.len() as f32 / 16000.0;
                        let filtered_duration = filtered.len() as f32 / 16000.0;
                        log::info!(
                            "VAD applied: original {:.2}s, filtered {:.2}s. ({:.1}% retained)",
                            original_duration,
                            filtered_duration,
                            (filtered_duration / original_duration) * 100.0,
                        );
                        filtered
                    }
                    Err(e) => {
                        log::error!("VAD processing failed: {}. Proceeding without VAD.", e);
                        samples_16k
                    }
                }
            } else {
                log::debug!("VAD model path not set. Skipping VAD.");
                samples_16k
            }
        } else {
            samples_16k
        };

        if samples_filtered.is_empty() {
            let mut state = self.state.lock().unwrap();
            *state = ManagerState::Idle;
            return Err(anyhow::anyhow!("No speech detected in the recording"));
        }

        // Transcribe based on engine type
        let result = match model_info.engine_type {
            EngineType::Cloud => {
                log::info!("Using cloud transcription (OpenAI)");
                self.cloud_transcriber
                    .transcribe(samples_filtered, 16000, None)
                    .await
            }
            EngineType::Parakeet => {
                log::info!("Using local transcription ({})", model_info.name);
                // Local transcription is sync
                self.local_transcriber.transcribe(samples_filtered)
            }
        };

        // Reset state
        {
            let mut state = self.state.lock().unwrap();
            *state = ManagerState::Idle;
        }

        result
    }

    /// Filter audio using VAD to remove silence
    fn filter_with_vad(
        &self,
        samples: &[f32],
        vad_path: &PathBuf,
    ) -> Result<Vec<f32>, anyhow::Error> {
        use crate::vad::VoiceActivityDetector;

        let silero = SileroVad::new(vad_path, 0.5)?;
        let mut smoothed_vad = SmoothedVad::with_defaults(Box::new(silero));

        let mut speech_samples = Vec::new();

        for chunk in samples.chunks(VAD_FRAME_SAMPLES) {
            let frame: Vec<f32> = if chunk.len() < VAD_FRAME_SAMPLES {
                let mut padded = chunk.to_vec();
                padded.resize(VAD_FRAME_SAMPLES, 0.0);
                padded
            } else {
                chunk.to_vec()
            };

            match smoothed_vad.push_frame(&frame)? {
                VadFrame::Speech(speech) => {
                    speech_samples.extend_from_slice(speech);
                }
                VadFrame::Noise => {
                    // Skip Silence
                }
            }
        }

        Ok(speech_samples)
    }

    pub fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        let mut recorder_guard = self.recorder.lock().unwrap();

        if let Some(recorder) = recorder_guard.as_mut() {
            let _ = recorder.stop();
            let _ = recorder.close();
        }
        *recorder_guard = None;
        *state = ManagerState::Idle;

        log::info!("Recording cancelled.");
    }

    pub fn unload_local_model(&self) {
        self.local_transcriber.unload_model();
    }
}

impl Drop for RecordingManager {
    fn drop(&mut self) {
        self.cancel();
        self.local_transcriber.unload_model();
    }
}

fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    let ratio = 16000.0 / from_rate as f64;
    let new_len = (samples.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = i as f64 / ratio;
        let idx_floor = src_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(samples.len() - 1);
        let frac = src_idx - idx_floor as f64;

        let sample = samples[idx_floor] as f64 * (1.0 - frac) + samples[idx_ceil] as f64 * frac;
        output.push(sample as f32);
    }

    output
}
