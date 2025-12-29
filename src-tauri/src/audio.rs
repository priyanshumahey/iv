//! Audio recorder that:
//! 1. Opens a microphone stream
//! 2. Records audio samples when started
//! 3. Returns samples when stopped
//! 4. Emits audio level updates during recording

use std::sync::{mpsc, Arc, Mutex};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SizedSample, Stream,
};

enum RecorderCommand {
    // Start recording - clear buffer and begin capturing
    Start,
    // Stop recording - return captured samples via the channel
    Stop(mpsc::Sender<Vec<f32>>),
    // Shutdown worker thread
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecorderState {
    Idle,
    Recording,
    Processing,
}

/// Callback for audio level updates (0.0 to 1.0)
pub type AudioLevelCallback = Arc<dyn Fn(f32) + Send + Sync>;

pub struct AudioRecorder {
    device: Option<Device>,
    cmd_tx: Option<mpsc::Sender<RecorderCommand>>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
    sample_rate: Arc<Mutex<u32>>,
    audio_level_callback: Option<AudioLevelCallback>,
}

impl AudioRecorder {
    /// Create a new audio recorder
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(AudioRecorder {
            device: None,
            cmd_tx: None,
            worker_handle: None,
            sample_rate: Arc::new(Mutex::new(16000)),
            audio_level_callback: None,
        })
    }

    /// Set the callback for audio level updates
    pub fn set_audio_level_callback<F>(&mut self, callback: F)
    where
        F: Fn(f32) + Send + Sync + 'static,
    {
        self.audio_level_callback = Some(Arc::new(callback));
    }

    /// Open the audio stream with the specified device (or default if None)
    pub fn open(&mut self, device: Option<Device>) -> Result<(), anyhow::Error> {
        if self.worker_handle.is_some() {
            log::debug!("AudioRecorder already open");
            return Ok(());
        }

        // Create channels for communication
        let (sample_tx, sample_rx) = mpsc::channel::<Vec<f32>>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<RecorderCommand>();

        // Get the host and device
        let host = cpal::default_host();
        let device = match device {
            Some(dev) => dev,
            None => host
                .default_input_device()
                .ok_or_else(|| anyhow::anyhow!("No default input device available"))?,
        };

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        log::info!("Using audio device: {}", device_name);

        // Get the best config for this device
        let config = Self::get_perferred_config(&device)?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        // Store the sample rate
        *self.sample_rate.lock().unwrap() = sample_rate;

        log::info!(
            "Audio config: {} Hz, {} channel(s), format: {:?}",
            sample_rate,
            channels,
            config.sample_format()
        );

        // Clone device for the thread
        let thread_device = device.clone();

        // Clone the audio level callback for the worker thread
        let level_callback = self.audio_level_callback.clone();

        // Spawn worker thread
        let worker = std::thread::spawn(move || {
            // Build stream based on sample format
            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    Self::build_stream::<f32>(&thread_device, &config, sample_tx.clone(), channels)
                }
                cpal::SampleFormat::I16 => {
                    Self::build_stream::<i16>(&thread_device, &config, sample_tx.clone(), channels)
                }
                cpal::SampleFormat::I32 => {
                    Self::build_stream::<i32>(&thread_device, &config, sample_tx.clone(), channels)
                }
                cpal::SampleFormat::U8 => {
                    Self::build_stream::<u8>(&thread_device, &config, sample_tx.clone(), channels)
                }
                format => {
                    log::error!("Unsupported sample format: {:?}", format);
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to build audio stream: {}", e);
                    return;
                }
            };

            // Start the stream
            if let Err(e) = stream.play() {
                log::error!("Failed to play audio stream: {}", e);
                return;
            }

            log::info!("Audio stream started");

            run_recording_loop(sample_rx, cmd_rx, level_callback);

            log::info!("Audio worker thread exiting");
        });

        self.device = Some(device);
        self.cmd_tx = Some(cmd_tx);
        self.worker_handle = Some(worker);

        Ok(())
    }

    /// Start recording audio
    pub fn start(&self) -> Result<(), anyhow::Error> {
        if let Some(tx) = &self.cmd_tx {
            tx.send(RecorderCommand::Start)?;
            log::debug!("Sent Start command to AudioRecorder");
        } else {
            return Err(anyhow::anyhow!("AudioRecorder not opened"));
        }
        Ok(())
    }

    /// Stop recording and return the captured samples
    pub fn stop(&self) -> Result<Vec<f32>, anyhow::Error> {
        let (resp_tx, resp_rx) = mpsc::channel();
        if let Some(tx) = &self.cmd_tx {
            tx.send(RecorderCommand::Stop(resp_tx))?;
        } else {
            return Err(anyhow::anyhow!("Recorder not opened"));
        }

        let samples = resp_rx.recv()?;
        log::debug!("Received {} samples from AudioRecorder", samples.len());
        Ok(samples)
    }

    /// Close the audio stream and clean it up
    pub fn close(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(RecorderCommand::Shutdown);
        }

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        self.device = None;
        log::debug!("AudioRecorder closed");
        Ok(())
    }

    /// Get the same rate of the recording
    pub fn sample_rate(&self) -> u32 {
        *self.sample_rate.lock().unwrap()
    }

    /// Build an input stream for the given sample type
    fn build_stream<T>(
        device: &Device,
        config: &cpal::SupportedStreamConfig,
        sample_tx: mpsc::Sender<Vec<f32>>,
        channels: usize,
    ) -> Result<Stream, cpal::BuildStreamError>
    where
        T: Sample + SizedSample + Send + 'static,
        f32: cpal::FromSample<T>,
    {
        let stream_config: cpal::StreamConfig = config.clone().into();

        device.build_input_stream(
            &stream_config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                // Convert samples to f32 and mono
                let mono_samples: Vec<f32> = if channels == 1 {
                    data.iter().map(|&s| s.to_sample::<f32>()).collect()
                } else {
                    data.chunks(channels)
                        .map(|frame| {
                            let sum: f32 = frame.iter().map(|&s| s.to_sample::<f32>()).sum();
                            sum / channels as f32
                        })
                        .collect()
                };

                // Send samples to the recording loop
                if sample_tx.send(mono_samples).is_err() {
                    // This is expected when the stream is closing - the receiver has been dropped
                    log::debug!("Audio channel closed, stream is shutting down");
                }
            },
            |err| {
                log::error!("Audio stream error: {}", err);
            },
            None, // No timeout
        )
    }

    /// Get the preferred audio configuration for a device
    fn get_perferred_config(device: &Device) -> Result<cpal::SupportedStreamConfig, anyhow::Error> {
        let supported_configs = device.supported_input_configs()?;

        let preferred_rates = [16000, 44100, 48000, 220050, 8000];

        let mut best_config: Option<cpal::SupportedStreamConfigRange> = None;

        for config_range in supported_configs {
            for &rate in &preferred_rates {
                if config_range.min_sample_rate().0 <= rate
                    && config_range.max_sample_rate().0 >= rate
                {
                    let should_use = match &best_config {
                        None => true,
                        Some(current) => {
                            let score = |fmt: cpal::SampleFormat| match fmt {
                                cpal::SampleFormat::F32 => 3,
                                cpal::SampleFormat::I16 => 2,
                                _ => 1,
                            };
                            score(config_range.sample_format()) > score(current.sample_format())
                        }
                    };

                    if should_use {
                        best_config = Some(config_range);
                        break;
                    }
                }
            }
        }

        if let Some(config) = best_config {
            for &rate in &preferred_rates {
                if config.min_sample_rate().0 <= rate && config.max_sample_rate().0 >= rate {
                    return Ok(config.with_sample_rate(cpal::SampleRate(rate)));
                }
            }
        }

        log::warn!("No preferred config found, using default");
        Ok(device.default_input_config()?)
    }
}

/// Calculate RMS (Root Mean Square) audio level from samples
/// Returns a value between 0.0 and 1.0
fn calculate_audio_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    // Calculate RMS
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    let rms = (sum_squares / samples.len() as f32).sqrt();

    // Convert to a more perceptually linear scale (0-1)
    // RMS values are typically very small (0.0 - 0.3 for normal speech)
    // We scale and clamp to get a useful 0-1 range
    let scaled = (rms * 4.0).min(1.0);

    // Apply slight curve for better visual response
    scaled.powf(0.7)
}

fn run_recording_loop(
    sample_rx: mpsc::Receiver<Vec<f32>>,
    cmd_rx: mpsc::Receiver<RecorderCommand>,
    level_callback: Option<AudioLevelCallback>,
) {
    let mut is_recording = false;
    let mut buffer: Vec<f32> = Vec::new();
    let mut level_sample_buffer: Vec<f32> = Vec::new();
    let mut last_level_update = std::time::Instant::now();
    const LEVEL_UPDATE_INTERVAL_MS: u64 = 33; // ~30fps

    loop {
        match sample_rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(samples) => {
                if is_recording {
                    buffer.extend(&samples);

                    // Accumulate samples for level calculation
                    if level_callback.is_some() {
                        level_sample_buffer.extend(&samples);

                        // Emit level updates at regular intervals
                        if last_level_update.elapsed().as_millis() >= LEVEL_UPDATE_INTERVAL_MS as u128 {
                            let level = calculate_audio_level(&level_sample_buffer);
                            if let Some(ref callback) = level_callback {
                                callback(level);
                            }
                            level_sample_buffer.clear();
                            last_level_update = std::time::Instant::now();
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No samples received, continue
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Stream closed
                log::debug!("sample_rx disconnected, exiting recording loop");
                break;
            }
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                RecorderCommand::Start => {
                    buffer.clear();
                    level_sample_buffer.clear();
                    is_recording = true;
                    log::debug!("Recording started in worker");
                }
                RecorderCommand::Stop(reply_tx) => {
                    is_recording = false;
                    let samples = std::mem::take(&mut buffer);
                    level_sample_buffer.clear();
                    log::debug!("Recording stopped in worker, captured {} samples", samples.len());
                    let _ = reply_tx.send(samples);
                }
                RecorderCommand::Shutdown => {
                    log::debug!("Shutdown command received, exiting recording loop");
                    return;
                }
            }
        }
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

pub fn list_input_devices() -> Result<Vec<String>, anyhow::Error> {
    let host = cpal::default_host();
    let devices = host.input_devices()?;

    let names: Vec<String> = devices.filter_map(|d| d.name().ok()).collect();

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        let devices = list_input_devices();
        println!("Available input devices: {:?}", devices);
    }
}
