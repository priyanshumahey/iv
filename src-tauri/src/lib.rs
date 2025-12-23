mod audio;
mod cloud_transcribe;
mod local_transcribe;
mod models;
mod recording_manager;
mod shortcut;
mod vad;
mod tray;
mod settings;

use std::sync::Arc;

use models::{ModelInfo, ModelManager};
use recording_manager::RecordingManager;
use tauri::{Manager, AppHandle};

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

#[tauri::command]
fn get_available_models(model_manager: tauri::State<Arc<ModelManager>>) -> Vec<ModelInfo> {
    model_manager.get_available_models()
}

#[tauri::command]
fn get_selected_model(manager: tauri::State<Arc<RecordingManager>>) -> String {
    manager.get_selected_model()
}

#[tauri::command]
fn set_selected_model(
    model_id: String,
    manager: tauri::State<Arc<RecordingManager>>,
) -> Result<(), String> {
    manager
        .set_selected_model(&model_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_model_downloaded(model_id: String, model_manager: tauri::State<Arc<ModelManager>>) -> bool {
    model_manager.is_model_downloaded(&model_id)
}

#[tauri::command]
async fn download_model(
    model_id: String,
    model_manager: tauri::State<'_, Arc<ModelManager>>,
) -> Result<(), String> {
    model_manager
        .download_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_model(
    model_id: String,
    model_manager: tauri::State<Arc<ModelManager>>,
) -> Result<(), String> {
    model_manager
        .delete_model(&model_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn unload_model(manager: tauri::State<Arc<RecordingManager>>) {
    manager.unload_local_model();
}

#[tauri::command]
fn is_vad_enabled(manager: tauri::State<Arc<RecordingManager>>) -> bool {
    manager.is_vad_enabled()
}

#[tauri::command]
fn set_vad_enabled(enabled: bool, manager: tauri::State<Arc<RecordingManager>>) {
    manager.set_vad_enabled(enabled);
}

#[tauri::command]
async fn ensure_vad_model(manager: tauri::State<'_, Arc<RecordingManager>>) -> Result<String, String> {
    manager
        .ensure_vad_model()
        .await
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_vad_model_downloaded(app_handle: AppHandle) -> bool {
    vad::is_vad_model_downloaded(&app_handle)
}

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
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log::info!("App starting up...");

            let model_manager = Arc::new(
                ModelManager::new(app.handle()).expect("Failed to initialize ModelManager"),
            );
            app.manage(model_manager.clone());

            let recording_manager = Arc::new(
                RecordingManager::new(app.handle(), model_manager)
                    .expect("Failed to initialize RecordingManager"),
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
            get_available_models,
            get_selected_model,
            set_selected_model,
            is_model_downloaded,
            download_model,
            delete_model,
            unload_model,
            is_vad_enabled,
            set_vad_enabled,
            ensure_vad_model,
            is_vad_model_downloaded
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
