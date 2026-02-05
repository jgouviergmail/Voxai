use std::path::Path;

use serde::Serialize;

use crate::error::AppError;

pub mod whisper;

/// Trait for speech-to-text engines.
/// All methods are sync — transcription is CPU-bound and should be
/// called within `tauri::async_runtime::spawn_blocking()`.
pub trait SttEngine: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn load_model(&mut self, model_path: &Path) -> Result<(), AppError>;
    fn unload_model(&mut self);
    fn is_loaded(&self) -> bool;
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<TranscriptionResult, AppError>;
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub segments: Vec<Segment>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}
