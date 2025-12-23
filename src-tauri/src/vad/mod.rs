//! Voice Activity Detection (VAD) module

mod download;
mod silero;
mod smoothed;

use anyhow::Result;

pub use download::{ensure_vad_model, is_vad_model_downloaded};
pub use silero::SileroVad;
pub use smoothed::SmoothedVad;

/// Result of processing a single VAD frame
pub enum VadFrame<'a> {
    /// Speech detected - contains the audio samples
    Speech(&'a [f32]),
    /// No speech (silence or noise)
    Noise,
}

impl<'a> VadFrame<'a> {
    #[inline]
    pub fn is_speech(&self) -> bool {
        matches!(self, VadFrame::Speech(_))
    }
}

/// Common trait for voice activity detection
pub trait VoiceActivityDetector: Send + Sync {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>>;

    fn is_voice(&mut self, frame: &[f32]) -> Result<bool> {
        Ok(self.push_frame(frame)?.is_speech())
    }

    fn reset(&mut self) {}
}

/// Frame size for Silero VAD at 16kHz (30ms)
pub const VAD_FRAME_SAMPLES: usize = 480; // 16000 * 30 / 1000
