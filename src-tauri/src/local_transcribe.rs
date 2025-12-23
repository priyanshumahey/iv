//! Local transcription engine using transcribe-rs

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use transcribe_rs::{
    engines::parakeet::{ParakeetEngine, ParakeetInferenceParams, ParakeetModelParams},
    TranscriptionEngine,
};

use crate::models::{EngineType, ModelInfo};

pub struct LocalTranscriber {
    engine: Mutex<Option<ParakeetEngine>>,
    current_model_id: Mutex<Option<String>>,
}

impl LocalTranscriber {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
            current_model_id: Mutex::new(None),
        }
    }

    /// Check if a model is currently loaded
    pub fn is_loaded(&self) -> bool {
        self.engine.lock().unwrap().is_some()
    }

    /// Get the currently loaded model ID
    pub fn current_model(&self) -> Option<String> {
        self.current_model_id.lock().unwrap().clone()
    }

    /// Load a model for transcription
    pub fn load_model(&self, model_info: &ModelInfo, model_path: &PathBuf) -> Result<()> {
        let load_start = std::time::Instant::now();
        log::info!("Loading model '{}' from {:?}", model_info.id, model_path);

        self.unload_model();

        if model_info.engine_type != EngineType::Parakeet {
            return Err(anyhow::anyhow!(
                "Only Parakeet models are supported for local transcription. Model '{}' is {:?}",
                model_info.id,
                model_info.engine_type
            ));
        }

        let mut engine = ParakeetEngine::new();
        engine
            .load_model_with_params(model_path, ParakeetModelParams::int8())
            .map_err(|e| anyhow::anyhow!("Failed to load Parakeet model: {}", e))?;

        // Store the loaded engine
        {
            let mut engine_guard = self.engine.lock().unwrap();
            *engine_guard = Some(engine);
        }
        {
            let mut model_id_guard = self.current_model_id.lock().unwrap();
            *model_id_guard = Some(model_info.id.clone());
        }

        let load_time = load_start.elapsed();
        log::info!(
            "Model '{}' loaded successfully in {}ms",
            model_info.id,
            load_time.as_millis()
        );

        Ok(())
    }

    /// Unload the current model to free memory
    pub fn unload_model(&self) {
        let mut engine_guard = self.engine.lock().unwrap();
        if let Some(ref mut engine) = *engine_guard {
            engine.unload_model();
        }
        *engine_guard = None;

        let mut model_id_guard = self.current_model_id.lock().unwrap();
        *model_id_guard = None;

        log::info!("Model unloaded");
    }

    /// Transcribe audio samples
    pub fn transcribe(&self, samples: Vec<f32>) -> Result<String> {
        if samples.is_empty() {
            log::debug!("Empty audio samples, returning empty string");
            return Ok(String::new());
        }

        let transcribe_start = std::time::Instant::now();
        let duration_secs = samples.len() as f32 / 16000.0;
        log::debug!(
            "Transcribing {} samples ({:.2}s of audio)",
            samples.len(),
            duration_secs
        );

        let mut engine_guard = self.engine.lock().unwrap();
        let engine = engine_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;

        let params = ParakeetInferenceParams::default();

        let result = engine
            .transcribe_samples(samples, Some(params))
            .map_err(|e| anyhow::anyhow!("Parakeet transcription failed: {}", e))?;

        let transcribe_time = transcribe_start.elapsed();
        let realtime_factor = duration_secs / transcribe_time.as_secs_f32();

        log::info!(
            "Transcription completed in {}ms ({:.1}x realtime): '{}'",
            transcribe_time.as_millis(),
            realtime_factor,
            result.text.trim()
        );

        Ok(result.text.trim().to_string())
    }
}

impl Default for LocalTranscriber {
    fn default() -> Self {
        Self::new()
    }
}
