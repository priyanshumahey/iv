//! Recording Manager - Orchestrates audio recording and transcription

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::audio::AudioRecorder;
use crate::cloud_transcribe::CloudTranscriber;
use crate::local_transcribe::LocalTranscriber;
use crate::models::{EngineType, ModelManager};
use crate::shortcut::events;

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
            log::warn!("Cannot start recording: currently in state {:?}", *state);
            return Ok(());
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

        log::debug!(
            "Recorded {} samples at {} Hz ({:.2}s)",
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

        // Resample to 16kHz if needed (required for all models)
        let samples_16k = if sample_rate != 16000 {
            log::debug!("Resampling from {} Hz to 16000 Hz", sample_rate);
            resample_to_16k(&samples, sample_rate)
        } else {
            samples
        };

        // Transcribe based on engine type
        let result = match model_info.engine_type {
            EngineType::Cloud => {
                log::info!("Using cloud transcription (OpenAI)");
                self.cloud_transcriber
                    .transcribe(samples_16k, 16000, None)
                    .await
            }
            EngineType::Parakeet => {
                log::info!("Using local transcription ({})", model_info.name);
                // Local transcription is sync
                self.local_transcriber.transcribe(samples_16k)
            }
        };

        // Reset state
        {
            let mut state = self.state.lock().unwrap();
            *state = ManagerState::Idle;
        }

        result
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
