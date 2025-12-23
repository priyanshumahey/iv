//! Audio feedback for recording start/stop sounds

use crate::settings;
use log::{debug, error, warn};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::thread;
use tauri::{AppHandle, Manager};

/// Types of feedback sounds
pub enum SoundType {
    Start,
    Stop,
}

/// Get the sound file path for a given sound type
fn get_sound_path(sound_type: &SoundType) -> &'static str {
    match sound_type {
        SoundType::Start => "resources/sounds/start.wav",
        SoundType::Stop => "resources/sounds/stop.wav",
    }
}

/// Resolve the full path to a sound file
fn resolve_sound_path(app: &AppHandle, sound_type: &SoundType) -> Option<PathBuf> {
    let sound_file = get_sound_path(sound_type);
    app.path()
        .resolve(sound_file, tauri::path::BaseDirectory::Resource)
        .ok()
}

/// Play a feedback sound asynchronously (non-blocking)
pub fn play_feedback_sound(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);

    if !settings.audio_feedback {
        return;
    }

    if let Some(path) = resolve_sound_path(app, &sound_type) {
        let volume = settings.audio_feedback_volume;
        play_sound_async(path, volume);
    } else {
        warn!(
            "Could not resolve sound path for {:?}",
            get_sound_path(&sound_type)
        );
    }
}

/// Play a feedback sound and block until complete
pub fn play_feedback_sound_blocking(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);

    if !settings.audio_feedback {
        return;
    }

    if let Some(path) = resolve_sound_path(app, &sound_type) {
        let volume = settings.audio_feedback_volume;
        play_sound_blocking(&path, volume);
    } else {
        warn!(
            "Could not resolve sound path for {:?}",
            get_sound_path(&sound_type)
        );
    }
}

/// Play a test sound (ignores audio_feedback setting)
pub fn play_test_sound(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);

    if let Some(path) = resolve_sound_path(app, &sound_type) {
        let volume = settings.audio_feedback_volume;
        play_sound_blocking(&path, volume);
    }
}

/// Play sound asynchronously in a separate thread
fn play_sound_async(path: PathBuf, volume: f32) {
    thread::spawn(move || {
        if let Err(e) = play_audio_file(&path, volume) {
            error!("Failed to play sound '{}': {}", path.display(), e);
        }
    });
}

/// Play sound and block until complete
fn play_sound_blocking(path: &PathBuf, volume: f32) {
    if let Err(e) = play_audio_file(path, volume) {
        error!("Failed to play sound '{}': {}", path.display(), e);
    }
}

/// Play an audio file using rodio
fn play_audio_file(path: &PathBuf, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Playing audio file: {}", path.display());

    let (_stream, stream_handle) = OutputStream::try_default()?;

    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let source = Decoder::new(buf_reader)?;
    sink.set_volume(volume);
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
