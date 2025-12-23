//! Silero VAD - High-quality voice activity detection using vad-rs
//!
//! Silero VAD is a lightweight neural network that provides accurate speech detection.
//! Model: https://github.com/snakers4/silero-vad

use std::path::Path;

use anyhow::Result;
use vad_rs::Vad;

use super::{VadFrame, VoiceActivityDetector, VAD_FRAME_SAMPLES};

/// Sample rate expected by Silero VAD
const SAMPLE_RATE: usize = 16000;

pub struct SileroVad {
    engine: Vad,
    threshold: f32,
}

impl SileroVad {
    /// Create a new Silero VAD instance from a model file
    pub fn new<P: AsRef<Path>>(model_path: P, threshold: f32) -> Result<Self> {
        if !(0.0..=1.0).contains(&threshold) {
            anyhow::bail!("threshold must be between 0.0 and 1.0");
        }

        log::info!("Loading Silero VAD model from {:?}", model_path.as_ref());

        let engine = Vad::new(&model_path, SAMPLE_RATE)
            .map_err(|e| anyhow::anyhow!("Failed to create VAD: {}", e))?;

        log::info!("Silero VAD loaded successfully");

        Ok(Self { engine, threshold })
    }
}

impl VoiceActivityDetector for SileroVad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        if frame.len() != VAD_FRAME_SAMPLES {
            anyhow::bail!(
                "expected {} samples (30ms at 16kHz), got {}",
                VAD_FRAME_SAMPLES,
                frame.len()
            );
        }

        let result = self
            .engine
            .compute(frame)
            .map_err(|e| anyhow::anyhow!("Silero VAD error: {}", e))?;

        if result.prob > self.threshold {
            Ok(VadFrame::Speech(frame))
        } else {
            Ok(VadFrame::Noise)
        }
    }
}
