//! Keyboard shortcut handling

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::recording_manager::RecordingManager;

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
            log::debug!("Shortcut pressed - starting recording");
            if let Err(e) = manager.start_recording() {
                log::error!("Failed to start recording: {}", e);
                let _ = app.emit(events::TRANSCRIPTION_ERROR, e.to_string());
            }
        }
        ShortcutState::Released => {
            log::debug!("Shortcut released - stopping recording");
            let manager = Arc::clone(&manager);
            let app_handle = app.clone();

            tauri::async_runtime::spawn(async move {
                let _ = app_handle.emit(events::TRANSCRIPTION_STARTED, ());

                tauri::async_runtime::spawn(async move {
                    let _ = app_handle.emit(events::TRANSCRIPTION_STARTED, ());

                    match manager.stop_and_transcribe().await {
                        Ok(text) => {
                            log::info!("Transcription complete: {}", text);
                            let _ = app_handle.emit(events::TRANSCRIPTION_COMPLETED, &text);
                        }
                        Err(e) => {
                            log::error!("Transcription error: {}", e);
                            let _ = app_handle.emit(events::TRANSCRIPTION_ERROR, e.to_string());
                        }
                    }
                });
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
