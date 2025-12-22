// This module coordinates:
// 1. Audio recording (via AudioRecorder)
// 2. Cloud transcription (via CloudTranscriber)
// 3. State management (idle, recording, transcribing)

use std::sync::{Arc, Mutex};

use crate::shortcut::events;
use async_openai::Audio;
use log::Record;
use tauri::{AppHandle, Emitter};

use crate::audio::AudioRecorder;
use crate::cloud_transcribe::CloudTranscriber;

#[derive(Clone, Debug, PartialEq)]
pub enum ManagerState {
    Idle,
    Recording,
    Transcribing,
}

pub struct RecordingManager {
    state: Mutex<ManagerState>,
    recorder: Mutex<Option<AudioRecorder>>,
    transcriber: CloudTranscriber,
    app_handle: AppHandle,
}

impl RecordingManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self, anyhow::Error> {
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        
        if api_key.is_none() {
            log::warn!("OPENAI_API_KEY is not set! Cloud transcription will be disabled.")
        }

        Ok(Self {
            state: Mutex::new(ManagerState::Idle),
            recorder: Mutex::new(None),
            transcriber: CloudTranscriber::new(api_key),
            app_handle: app_handle.clone(),
        })
    }

    pub fn get_state(&self) -> ManagerState {
        self.state.lock().unwrap().clone()
    }

    pub fn start_recording(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if *state != ManagerState::Idle {
            log::warn!("Cannot start recording: current state is {:?}", *state);
            return Ok(());
        }

        let mut recorder = AudioRecorder::new()?;
        recorder.open(None);
        recorder.start()?;

        *self.recorder.lock().unwrap() = Some(recorder);
        *state = ManagerState::Recording;

        let _ = self.app_handle.emit(events::RECORDING_STARTED, ());

        log::info!("Recording started.");
        Ok(())
    }

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
            return Err(anyhow::anyhow!("no audio recorded"));
        }

        let result = self
            .transcriber
            .transcribe(samples, sample_rate, None)
            .await;

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
}

impl Drop for RecordingManager {
    fn drop(&mut self) {
        self.cancel();
    }
}
