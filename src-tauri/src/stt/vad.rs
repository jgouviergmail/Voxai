use std::path::PathBuf;

use whisper_rs::{WhisperVadContext, WhisperVadContextParams, WhisperVadParams};

use crate::error::AppError;

/// Silero VAD model embedded in the binary (~885KB).
const SILERO_VAD_MODEL: &[u8] = include_bytes!("../../models/ggml-silero-v6.2.0.bin");

/// Ensure the Silero VAD model is extracted to disk.
/// Returns the file path for `WhisperVadContext::new()`.
pub fn ensure_vad_model() -> Result<PathBuf, AppError> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| AppError::Internal("Cannot determine local data dir".into()))?
        .join("Voxai")
        .join("models");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("ggml-silero-v6.2.0.bin");
    if !path.exists() {
        std::fs::write(&path, SILERO_VAD_MODEL)?;
        log::info!("Silero VAD model extracted to {:?}", path);
    }
    Ok(path)
}

/// Validate whether a 16kHz mono audio segment contains speech.
/// Creates a WhisperVadContext per call (~5ms overhead for 885KB model).
/// Must be called from a blocking thread (WhisperVadContext is !Send).
pub fn validate_speech(model_path: &str, samples: &[f32]) -> Result<bool, AppError> {
    let params = WhisperVadContextParams::new();
    let mut ctx = WhisperVadContext::new(model_path, params)
        .map_err(|e| AppError::Stt(format!("VAD init failed: {e}")))?;

    let mut vad_params = WhisperVadParams::new();
    vad_params.set_threshold(0.5);
    vad_params.set_min_speech_duration(250);  // 250ms min speech
    vad_params.set_speech_pad(200);           // 200ms padding around speech

    let segments = ctx
        .segments_from_samples(vad_params, samples)
        .map_err(|e| AppError::Stt(format!("VAD failed: {e}")))?;

    let n = segments.num_segments();
    if n > 0 {
        log::info!("[VAD] {} speech segment(s) detected", n);
    } else {
        log::info!(
            "[VAD] no speech in {} samples ({:.1}s)",
            samples.len(),
            samples.len() as f64 / 16000.0
        );
    }

    Ok(n > 0)
}
