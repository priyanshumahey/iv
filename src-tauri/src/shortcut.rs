//! Keyboard shortcut handling with full UX integration

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::audio_feedback::{self, SoundType};
use crate::clipboard;
use crate::overlay::{self, OverlayState};
use crate::recording_manager::RecordingManager;
use crate::tray::{self, TrayIconState};

pub const DEFAULT_SHORTCUT: &str = "ctrl+space";

pub mod events {
    pub const RECORDING_STARTED: &str = "recording-started";
    pub const RECORDING_STOPPED: &str = "recording-stopped";
    pub const TRANSCRIPTION_STARTED: &str = "transcription-started";
    pub const TRANSCRIPTION_COMPLETED: &str = "transcription-completed";
    pub const TRANSCRIPTION_ERROR: &str = "transcription-error";
}

pub fn init_shortcut(app: &AppHandle) -> Result<(), String> {
    let shortcut_str = DEFAULT_SHORTCUT;

    let shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|e| format!("Failed to parse shortcut '{}': {}", shortcut_str, e))?;

    if app.global_shortcut().is_registered(shortcut) {
        log::warn!("Shortcut '{}' is already registered", shortcut_str);
        return Ok(());
    }

    app.global_shortcut()
        .on_shortcut(shortcut, move |app_handle, _shortcut, event| {
            handle_shortcut_event(app_handle, event.state);
        })
        .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut_str, e))?;

    log::info!("Registered global shortcut: '{}'", shortcut_str);
    Ok(())
}

fn handle_shortcut_event(app: &AppHandle, state: ShortcutState) {
    let manager = match app.try_state::<Arc<RecordingManager>>() {
        Some(m) => m,
        None => {
            log::error!("RecordingManager not found in app state");
            return;
        }
    };

    match state {
        ShortcutState::Pressed => {
            log::debug!("Shortcut pressed - attempting to start recording");

            // Try to start recording first - this will fail if we're currently transcribing
            if let Err(e) = manager.start_recording() {
                log::warn!("Cannot start recording: {}", e);
                // Don't update UI or play sounds if we can't start recording
                return;
            }

            // Only update UI after recording has successfully started
            tray::change_tray_icon(app, TrayIconState::Recording);
            overlay::show_overlay(app, OverlayState::Recording);
            audio_feedback::play_feedback_sound(app, SoundType::Start);
        }
        ShortcutState::Released => {
            log::debug!("Shortcut released - stopping recording");

            let manager = Arc::clone(&manager);
            let app_handle = app.clone();

            tauri::async_runtime::spawn(async move {
                // Update UI to transcribing state
                tray::change_tray_icon(&app_handle, TrayIconState::Transcribing);
                overlay::update_overlay_state(&app_handle, OverlayState::Transcribing);

                let _ = app_handle.emit(events::TRANSCRIPTION_STARTED, ());

                match manager.stop_and_transcribe().await {
                    Ok(text) => {
                        log::info!("Transcription complete: {}", text);

                        // Play stop sound
                        audio_feedback::play_feedback_sound(&app_handle, SoundType::Stop);

                        // Emit completion event to frontend
                        let _ = app_handle.emit(events::TRANSCRIPTION_COMPLETED, &text);

                        // Paste the transcribed text
                        if let Err(e) = clipboard::paste(text, &app_handle) {
                            log::error!("Failed to paste transcription: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("Transcription error: {}", e);
                        let _ = app_handle.emit(events::TRANSCRIPTION_ERROR, e.to_string());
                    }
                }

                // Reset UI
                tray::change_tray_icon(&app_handle, TrayIconState::Idle);
                overlay::hide_overlay(&app_handle);
            });
        }
    }
}

pub fn cleanup_shortcut(app: &AppHandle) {
    let shortcut: Result<Shortcut, _> = DEFAULT_SHORTCUT.parse();
    if let Ok(s) = shortcut {
        let _ = app.global_shortcut().unregister(s);
        log::debug!("Unregistered global shortcut: '{}'", DEFAULT_SHORTCUT);
    }
}
