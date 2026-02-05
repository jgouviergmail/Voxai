use serde::Serialize;

// Event name constants
pub const EVENT_RECORDING_STATE_CHANGED: &str = "recording-state-changed";
pub const EVENT_TRANSCRIPTION_COMPLETE: &str = "transcription-complete";
pub const EVENT_DOWNLOAD_PROGRESS: &str = "download-progress";
pub const EVENT_ERROR: &str = "app-error";

// Event payload types
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
}
