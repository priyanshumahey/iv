//! Cloud transcription module using OpenAI's whisper API

use std::io::Cursor;

use async_openai::{
    config::OpenAIConfig,
    types::{AudioInput, AudioResponseFormat, CreateTranscriptionRequestArgs},
    Client
};
use hound::{SampleFormat, WavSpec, WavWriter};

pub struct CloudTranscriber {
    client: Client<OpenAIConfig>,
}

impl CloudTranscriber {
    /// Create a new cloud transcriber
    pub fn new(api_key: Option<String>) -> Self {
        let client = match api_key {
            Some(key) => {
                let config = OpenAIConfig::new().with_api_key(key);
                Client::with_config(config)
            }
            None => Client::new(),
        };

        Self { client }
    }

    /// Transcribe audio samples
    pub async fn transcribe(
        &self,
        samples: Vec<f32>,
        sample_rate: u32,
        language: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        if samples.is_empty() {
            return Err(anyhow::anyhow!("No audio samples provided"));
        }

        let wav_bytes = samples_to_wav(&samples, sample_rate)?;

        log::debug!(
            "Sending {} bytes of audio to OpenAI ({} samples at {} Hz)",
            wav_bytes.len(),
            samples.len(),
            sample_rate
        );

        // Build the transcriptionr request
        let audio_input = AudioInput::from_vec_u8("audio.wav".to_string(), wav_bytes);

        let mut request_builder = CreateTranscriptionRequestArgs::default();
        request_builder
            .file(audio_input)
            .model("whisper-1")
            .response_format(AudioResponseFormat::Text);

        if let Some(lang) = language {
            request_builder.language(lang);
        }

        let request = request_builder.build()?;

        // Send request
        let response = self.client.audio().transcribe(request).await?;

        log::debug!("Transcription result: {}", response.text);
        Ok(response.text.trim().to_string())
    }
}

/// Convert f32 samples to WAV format bytes
fn samples_to_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, anyhow::Error> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int
    };

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut buffer, spec)?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let scaled = (clamped * 32767.0) as i16;
            writer.write_sample(scaled)?;
        }

        writer.finalize()?;
    }

    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples_to_wav() {
        let sample_rate = 16000;
        let duration_secs = 0.1;
        let num_samples = (sample_rate as f32 * duration_secs) as usize;

        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5
            })
            .collect();

        let wav_bytes = samples_to_wav(&samples, sample_rate).unwrap();

        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");

        println!("Generated WAV bytes length: {}", wav_bytes.len());
    }

    #[test]
    fn test_empty_samples() {
        let wav_bytes = samples_to_wav(&[], 16000).unwrap();
        assert!(wav_bytes.len() >= 44);
    }
}
