//! Application settings management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";

/// Shortcut binding configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_binding: String,
    pub current_binding: String,
}

/// Overlay position options
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OverlayPosition {
    None,
    Top,
    #[default]
    Bottom,
}

/// Paste method options
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    #[default]
    CtrlV,
    Direct,
    None,
    ShiftInsert,
    CtrlShiftV,
}

/// Clipboard handling options
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardHandling {
    #[default]
    DontModify,
    CopyToClipboard,
}

/// Main application settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    /// Keyboard shortcut bindings
    pub bindings: HashMap<String, ShortcutBinding>,

    /// Selected transcription model ID
    pub selected_model: String,

    /// Selected microphone device name (None = default)
    pub selected_input_device: Option<String>,

    /// Selected output device for audio feedback (None = default)
    pub selected_output_device: Option<String>,

    /// Whether VAD is enabled
    pub vad_enabled: bool,

    /// Whether audio feedback is enabled
    pub audio_feedback: bool,

    /// Audio feedback volume (0.0 - 1.0)
    pub audio_feedback_volume: f32,

    /// Overlay position
    pub overlay_position: OverlayPosition,

    /// Paste method to use
    pub paste_method: PasteMethod,

    /// Whether to append trailing space after transcription
    pub append_trailing_space: bool,

    /// Clipboard handling behavior
    pub clipboard_handling: ClipboardHandling,

    /// Transcription language (e.g., "en", "auto")
    pub transcription_language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Default push-to-talk shortcut
        let default_shortcut = if cfg!(target_os = "macos") {
            "Alt+Space"
        } else {
            "Ctrl+Space"
        };

        bindings.insert(
            "transcribe".to_string(),
            ShortcutBinding {
                id: "transcribe".to_string(),
                name: "Push to Talk".to_string(),
                description: "Hold to record, release to transcribe".to_string(),
                default_binding: default_shortcut.to_string(),
                current_binding: default_shortcut.to_string(),
            },
        );

        Self {
            bindings,
            selected_model: "cloud".to_string(),
            selected_input_device: None,
            selected_output_device: None,
            vad_enabled: true,
            audio_feedback: true,
            audio_feedback_volume: 0.5,
            overlay_position: OverlayPosition::Bottom,
            paste_method: PasteMethod::CtrlV,
            append_trailing_space: true,
            clipboard_handling: ClipboardHandling::DontModify,
            transcription_language: "en".to_string(),
        }
    }
}

/// Get current settings from the store, or defaults if not set
pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = match app.store(SETTINGS_STORE_PATH) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to get settings store: {}", e);
            return AppSettings::default();
        }
    };

    match store.get("settings") {
        Some(value) => match serde_json::from_value::<AppSettings>(value.clone()) {
            Ok(settings) => settings,
            Err(e) => {
                log::warn!("Failed to deserialize settings, using defaults: {}", e);
                AppSettings::default()
            }
        },
        None => {
            log::debug!("No settings found, using defaults");
            AppSettings::default()
        }
    }
}

/// Write settings to the store
pub fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|e| format!("Failed to get settings store: {}", e))?;

    let value = serde_json::to_value(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    store.set("settings", value);
    store
        .save()
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    log::debug!("Settings saved");
    Ok(())
}

/// Update a single setting field
pub fn update_setting<F>(app: &AppHandle, updater: F) -> Result<(), String>
where
    F: FnOnce(&mut AppSettings),
{
    let mut settings = get_settings(app);
    updater(&mut settings);
    write_settings(app, &settings)
}
