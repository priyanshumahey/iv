//! Model Manager - handles model discovery, downloading, and path resolution

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use tar::Archive;
use tauri::{AppHandle, Emitter, Manager};

use super::types::{DownloadProgress, EngineType, ModelInfo};

pub struct ModelManager {
    app_handle: AppHandle,
    models_dir: PathBuf,
    available_models: Mutex<HashMap<String, ModelInfo>>,
}

impl ModelManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let models_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?
            .join("models");

        // Create models directory if it doesn't exist
        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        log::info!("Models directory: {:?}", models_dir);

        let mut available_models = HashMap::new();

        // Cloud is always available
        let cloud = ModelInfo::cloud();
        available_models.insert(cloud.id.clone(), cloud);

        let parakeet_v3 = ModelInfo::parakeet_v3();
        available_models.insert(parakeet_v3.id.clone(), parakeet_v3);

        let manager = Self {
            app_handle: app_handle.clone(),
            models_dir,
            available_models: Mutex::new(available_models),
        };

        // Update download status for all models
        manager.refresh_download_status()?;

        Ok(manager)
    }

    /// Get all available models
    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        let models = self.available_models.lock().unwrap();
        models.values().cloned().collect()
    }

    /// Get info for a specific model
    pub fn get_model_info(&self, model_id: &str) -> Option<ModelInfo> {
        let models = self.available_models.lock().unwrap();
        models.get(model_id).cloned()
    }

    /// Get the path to a model's files
    pub fn get_model_path(&self, model_id: &str) -> Result<PathBuf> {
        let models = self.available_models.lock().unwrap();
        let model = models
            .get(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        if model.engine_type == EngineType::Cloud {
            return Err(anyhow::anyhow!("Cloud model has no local path"));
        }

        Ok(self.models_dir.join(&model.filename))
    }

    /// Check if a model is downloaded
    pub fn is_model_downloaded(&self, model_id: &str) -> bool {
        let models = self.available_models.lock().unwrap();
        models
            .get(model_id)
            .map(|m| m.is_downloaded)
            .unwrap_or(false)
    }

    /// Refresh the download status of all models
    pub fn refresh_download_status(&self) -> Result<()> {
        let mut models = self.available_models.lock().unwrap();

        for model in models.values_mut() {
            if model.engine_type == EngineType::Cloud {
                model.is_downloaded = true;
                continue;
            }

            let model_path = self.models_dir.join(&model.filename);

            if model.is_directory {
                // Directory-based models (Parakeet)
                model.is_downloaded = model_path.exists() && model_path.is_dir();

                // Clean up interrupted extractions
                let extracting_path = self
                    .models_dir
                    .join(format!("{}.extracting", &model.filename));
                if extracting_path.exists() {
                    log::warn!("Cleaning up interrupted extraction: {:?}", extracting_path);
                    let _ = fs::remove_dir_all(&extracting_path);
                }
            } else {
                // Single file models (Whisper)
                model.is_downloaded = model_path.exists() && model_path.is_file();
            }

            // Check for partial downloads
            let partial_path = self.models_dir.join(format!("{}.partial", &model.filename));
            if partial_path.exists() {
                model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
            } else {
                model.partial_size = 0;
            }

            model.is_downloading = false;

            log::debug!(
                "Model '{}': downloaded={}, partial_size={}",
                model.id,
                model.is_downloaded,
                model.partial_size
            );
        }

        Ok(())
    }

    /// Download a model
    pub async fn download_model(&self, model_id: &str) -> Result<()> {
        // Get model info
        let model = {
            let models = self.available_models.lock().unwrap();
            models
                .get(model_id)
                .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?
                .clone()
        };

        if model.engine_type == EngineType::Cloud {
            return Err(anyhow::anyhow!("Cloud model doesn't need downloading"));
        }

        let url = model
            .url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model has no download URL"))?;

        // Mark as downloading
        {
            let mut models = self.available_models.lock().unwrap();
            if let Some(m) = models.get_mut(model_id) {
                m.is_downloading = true;
            }
        }

        log::info!("Starting download of model '{}' from {}", model_id, url);

        // Emit download started event
        let _ = self.app_handle.emit(
            "model-download-started",
            serde_json::json!({ "model_id": model_id }),
        );

        let result = self.do_download(&model, url).await;

        // Mark as not downloading
        {
            let mut models = self.available_models.lock().unwrap();
            if let Some(m) = models.get_mut(model_id) {
                m.is_downloading = false;
            }
        }

        // Refresh status
        let _ = self.refresh_download_status();

        match &result {
            Ok(()) => {
                log::info!("Model '{}' downloaded successfully", model_id);
                let _ = self.app_handle.emit(
                    "model-download-complete",
                    serde_json::json!({ "model_id": model_id }),
                );
            }
            Err(e) => {
                log::error!("Failed to download model '{}': {}", model_id, e);
                let _ = self.app_handle.emit(
                    "model-download-error",
                    serde_json::json!({
                        "model_id": model_id,
                        "error": e.to_string()
                    }),
                );
            }
        }

        result
    }

    /// Internal download implementation
    async fn do_download(&self, model: &ModelInfo, url: &str) -> Result<()> {
        let client = reqwest::Client::new();

        // Determine paths
        let partial_path = if model.is_directory {
            self.models_dir
                .join(format!("{}.partial.tar.gz", &model.filename))
        } else {
            self.models_dir.join(format!("{}.partial", &model.filename))
        };

        // Clean up any failed extraction attempts
        if model.is_directory {
            let extracting_path = self
                .models_dir
                .join(format!("{}.extracting", &model.filename));
            if extracting_path.exists() {
                log::warn!("Cleaning up interrupted extraction: {:?}", extracting_path);
                let _ = fs::remove_dir_all(&extracting_path);
            }
        }

        // Check for existing partial download
        let existing_size = if partial_path.exists() {
            partial_path.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        // First, do a HEAD request to get the total file size
        let head_response = client.head(url).send().await?;
        let expected_size = head_response.content_length().unwrap_or(0);

        // Check if partial file is already complete
        let skip_download = existing_size > 0 && existing_size >= expected_size;

        if skip_download {
            log::info!(
                "Partial file is complete ({} bytes), skipping download",
                existing_size
            );
        } else {
            // Build request with range header for resume
            let mut request = client.get(url);
            if existing_size > 0 {
                log::info!("Resuming download from byte {}", existing_size);
                request = request.header("Range", format!("bytes={}-", existing_size));
            }

            let response = request.send().await?;

            // Check for success or partial content
            let status = response.status();
            if !status.is_success() && status.as_u16() != 206 {
                // If we get 416 Range Not Satisfiable, the file might be complete
                if status.as_u16() == 416 && existing_size > 0 {
                    log::info!(
                        "Server returned 416, assuming download is complete ({} bytes)",
                        existing_size
                    );
                } else {
                    return Err(anyhow::anyhow!("Download failed with status: {}", status));
                }
            } else {
                // Get total size
                let content_length = response.content_length().unwrap_or(0);
                let total_size = if status.as_u16() == 206 {
                    // Partial content - add existing size
                    existing_size + content_length
                } else {
                    content_length
                };

                log::info!(
                    "Downloading {} bytes (total: {})",
                    content_length,
                    total_size
                );

                // Open file for writing (append if resuming)
                let mut file = if existing_size > 0 && status.as_u16() == 206 {
                    fs::OpenOptions::new().append(true).open(&partial_path)?
                } else {
                    File::create(&partial_path)?
                };

                // Stream the download
                let mut stream = response.bytes_stream();
                let mut downloaded = existing_size;

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    file.write_all(&chunk)?;
                    downloaded += chunk.len() as u64;

                    // Emit progress every ~100KB
                    if downloaded % (100 * 1024) < chunk.len() as u64 {
                        let progress = DownloadProgress::new(&model.id, downloaded, total_size);
                        let _ = self.app_handle.emit("model-download-progress", &progress);
                    }
                }

                // Ensure all data is written
                file.flush()?;
                drop(file);

                // Emit final progress
                let progress = DownloadProgress::new(&model.id, downloaded, total_size);
                let _ = self.app_handle.emit("model-download-progress", &progress);
            }
        }

        // Handle directory models (extract tar.gz)
        if model.is_directory {
            self.extract_model(&partial_path, &model.filename)?;
            // Remove the archive
            let _ = fs::remove_file(&partial_path);
        } else {
            // Rename partial to final
            let final_path = self.models_dir.join(&model.filename);
            fs::rename(&partial_path, &final_path)?;
        }

        Ok(())
    }

    /// Extract a tar.gz archive to a model directory
    fn extract_model(&self, archive_path: &PathBuf, dir_name: &str) -> Result<()> {
        log::info!("Extracting model archive to '{}'", dir_name);

        // Extract to a temp directory first
        let extracting_path = self.models_dir.join(format!("{}.extracting", dir_name));
        if extracting_path.exists() {
            fs::remove_dir_all(&extracting_path)?;
        }
        fs::create_dir_all(&extracting_path)?;

        // Open and decompress
        let file = File::open(archive_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        // Extract
        archive.unpack(&extracting_path)?;

        // Find the actual model directory inside (might be nested)
        let final_path = self.models_dir.join(dir_name);
        if final_path.exists() {
            fs::remove_dir_all(&final_path)?;
        }

        // Check if there's a nested directory with the same name
        let nested_path = extracting_path.join(dir_name);
        if nested_path.exists() && nested_path.is_dir() {
            fs::rename(&nested_path, &final_path)?;
            fs::remove_dir_all(&extracting_path)?;
        } else {
            // Just rename the extracting dir
            fs::rename(&extracting_path, &final_path)?;
        }

        log::info!("Model extracted successfully");
        Ok(())
    }

    /// Delete a downloaded model
    pub fn delete_model(&self, model_id: &str) -> Result<()> {
        let model = {
            let models = self.available_models.lock().unwrap();
            models
                .get(model_id)
                .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?
                .clone()
        };

        if model.engine_type == EngineType::Cloud {
            return Err(anyhow::anyhow!("Cannot delete cloud model"));
        }

        let model_path = self.models_dir.join(&model.filename);

        if model_path.exists() {
            if model.is_directory {
                fs::remove_dir_all(&model_path)?;
            } else {
                fs::remove_file(&model_path)?;
            }
            log::info!("Deleted model '{}'", model_id);
        }

        // Also clean up any partial files
        let partial_path = self.models_dir.join(format!("{}.partial", &model.filename));
        let _ = fs::remove_file(&partial_path);

        let partial_tar_path = self
            .models_dir
            .join(format!("{}.partial.tar.gz", &model.filename));
        let _ = fs::remove_file(&partial_tar_path);

        self.refresh_download_status()?;

        Ok(())
    }
}
