//! Model types and data structures

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineType {
    Parakeet,
    Cloud,
}

impl Default for EngineType {
    fn default() -> Self {
        Self::Cloud
    }
}

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique identifier for the model
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of the model's characteristics
    pub description: String,
    /// Filename or directory name in the models folder
    pub filename: String,
    /// Download URL (None for cloud API)
    pub url: Option<String>,
    /// Approximate size in MB
    pub size_mb: u64,
    /// Whether the model is downloaded and ready
    pub is_downloaded: bool,
    /// Whether currently downloading
    pub is_downloading: bool,
    /// Partial download size (for resume)
    pub partial_size: u64,
    /// Whether this is a directory-based model (Parakeet) vs single file
    pub is_directory: bool,
    /// The engine type for this model
    pub engine_type: EngineType,
    /// Accuracy score (0.0 to 1.0, higher is better)
    pub accuracy_score: f32,
    /// Speed score (0.0 to 1.0, higher is faster)
    pub speed_score: f32,
}

impl ModelInfo {
    /// Create the cloud (OpenAI API) pseudo-model
    pub fn cloud() -> Self {
        Self {
            id: "cloud".to_string(),
            name: "OpenAI Cloud".to_string(),
            description:
                "Uses OpenAI's Whisper API. Requires internet and API key. Multi-language support."
                    .to_string(),
            filename: String::new(),
            url: None,
            size_mb: 0,
            is_downloaded: true, // Always "available"
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Cloud,
            accuracy_score: 0.95,
            speed_score: 0.70, // Depends on network
        }
    }
    
    pub fn parakeet_v3() -> Self {
        Self {
            id: "parakeet-v3".to_string(),
            name: "Parakeet V3 (Recommended)".to_string(),
            description: "English only. Best accuracy, latest version. Recommended for most users."
                .to_string(),
            filename: "parakeet-tdt-0.6b-v3-int8".to_string(),
            url: Some(
                "https://huggingface.co/tanerror/parakeet-v3/resolve/main/parakeet-v3-int8.tar.gz"
                    .to_string(),
            ),
            size_mb: 478,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Parakeet,
            accuracy_score: 0.92,
            speed_score: 0.85,
        }
    }
}

/// Download progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Model being downloaded
    pub model_id: String,
    /// Bytes downloaded so far
    pub downloaded: u64,
    /// Total bytes to download
    pub total: u64,
    /// Percentage complete (0.0 to 100.0)
    pub percentage: f64,
}

impl DownloadProgress {
    pub fn new(model_id: &str, downloaded: u64, total: u64) -> Self {
        let percentage = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            model_id: model_id.to_string(),
            downloaded,
            total,
            percentage,
        }
    }
}
