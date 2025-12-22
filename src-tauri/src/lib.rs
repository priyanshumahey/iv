mod audio;
mod cloud_transcribe;
mod recording_manager;
mod shortcut;

use std::sync::Arc;

use recording_manager::RecordingManager;
use tauri::Manager;

#[tauri::command]
fn get_recording_state(manager: tauri::State<Arc<RecordingManager>>) -> String {
    format!("{:?}", manager.get_state())
}

#[tauri::command]
fn cancel_recording(manager: tauri::State<Arc<RecordingManager>>) {
    manager.cancel();
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<String>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("No .env file loaded: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .level(log::LevelFilter::Info)
            .build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log::info!("App starting up...");
            let recording_manager = Arc::new(
                RecordingManager::new(app.handle()).expect("Failed to initialize RecordingManager"),
            );
            app.manage(recording_manager);

            if let Err(e) = shortcut::init_shortcut(app.handle()) {
                log::error!("Failed to initialize shortcut: {}", e);
            }

            log::info!("App setup complete.");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_recording_state,
            cancel_recording,
            list_audio_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
