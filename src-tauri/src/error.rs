use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Transcription error: {0}")]
    Stt(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Injection error: {0}")]
    Injection(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// -- From impls for crate errors --

impl From<cpal::DevicesError> for AppError {
    fn from(e: cpal::DevicesError) -> Self {
        Self::Audio(e.to_string())
    }
}

impl From<cpal::DeviceNameError> for AppError {
    fn from(e: cpal::DeviceNameError) -> Self {
        Self::Audio(e.to_string())
    }
}

impl From<cpal::DefaultStreamConfigError> for AppError {
    fn from(e: cpal::DefaultStreamConfigError) -> Self {
        Self::Audio(e.to_string())
    }
}

impl From<cpal::BuildStreamError> for AppError {
    fn from(e: cpal::BuildStreamError) -> Self {
        Self::Audio(e.to_string())
    }
}

impl From<cpal::PlayStreamError> for AppError {
    fn from(e: cpal::PlayStreamError) -> Self {
        Self::Audio(e.to_string())
    }
}

impl From<whisper_rs::WhisperError> for AppError {
    fn from(e: whisper_rs::WhisperError) -> Self {
        Self::Stt(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Config(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<arboard::Error> for AppError {
    fn from(e: arboard::Error) -> Self {
        Self::Injection(e.to_string())
    }
}
