//! VAD model download helper

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter, Manager};

pub const VAD_MODEL_NAME: &str = "silero_vad.onnx";

pub const VAD_MODEL_URL: &str =
    "https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx";

pub fn get_vad_model_path(app_handle: &AppHandle) -> Result<PathBuf> {
    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?
        .join("models");

    Ok(models_dir.join(VAD_MODEL_NAME))
}

pub fn is_vad_model_downloaded(app_handle: &AppHandle) -> bool {
    get_vad_model_path(app_handle)
        .map(|p| p.exists())
        .unwrap_or(false)
}

pub async fn ensure_vad_model(app_handle: &AppHandle) -> Result<PathBuf> {
    let model_path = get_vad_model_path(app_handle)?;

    if model_path.exists() {
        log::info!("VAD model already present at {:?}", model_path);
        return Ok(model_path);
    }

    log::info!("Downloading VAD model from {}", VAD_MODEL_URL);

    // Ensure models directory exists
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Emit download started event
    let _ = app_handle.emit("vad-model-download-started", ());

    // Download the model
    let client = reqwest::Client::new();
    let response = client.get(VAD_MODEL_URL).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download VAD model: HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;

    // Create temp file for download
    let temp_path = model_path.with_extension("onnx.tmp");
    let mut file = File::create(&temp_path)?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        // Emit progress event
        if total_size > 0 {
            let percentage = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            let _ = app_handle.emit(
                "vad-model-download-progress",
                serde_json::json!({
                    "downloaded": downloaded,
                    "total": total_size,
                    "percentage": percentage
                }),
            );
        }
    }

    file.flush()?;
    drop(file);

    // Rename temp file to final path
    fs::rename(&temp_path, &model_path)?;

    log::info!("VAD model downloaded to {:?}", model_path);

    // Emit download complete event
    let _ = app_handle.emit("vad-model-download-complete", ());

    Ok(model_path)
}
