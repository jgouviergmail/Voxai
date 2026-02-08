use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use whisper_rs::{get_lang_str, FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

use super::{Segment, SttEngine, TranscriptionResult};
use crate::error::AppError;

/// All languages supported by Whisper, as (ISO 639-1 code, English name).
pub const WHISPER_LANGUAGES: &[(&str, &str)] = &[
    ("af", "Afrikaans"),
    ("ar", "Arabic"),
    ("hy", "Armenian"),
    ("az", "Azerbaijani"),
    ("be", "Belarusian"),
    ("bs", "Bosnian"),
    ("bg", "Bulgarian"),
    ("ca", "Catalan"),
    ("zh", "Chinese"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("et", "Estonian"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("gl", "Galician"),
    ("de", "German"),
    ("el", "Greek"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("is", "Icelandic"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("ko", "Korean"),
    ("lv", "Latvian"),
    ("lt", "Lithuanian"),
    ("mk", "Macedonian"),
    ("ms", "Malay"),
    ("mr", "Marathi"),
    ("mi", "Maori"),
    ("ne", "Nepali"),
    ("no", "Norwegian"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sr", "Serbian"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("es", "Spanish"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tl", "Tagalog"),
    ("ta", "Tamil"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
];

/// Look up the English name for a language code.
pub fn language_name_from_code(code: &str) -> Option<&'static str> {
    WHISPER_LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
}

pub struct WhisperEngine {
    context: Option<WhisperContext>,
    /// Cached WhisperState for reuse across streaming segments.
    /// Avoids re-allocating ~200MB of compute buffers per segment.
    cached_state: Mutex<Option<WhisperState>>,
    use_gpu: bool,
    thread_limit: Arc<RwLock<Option<u32>>>,
}

impl WhisperEngine {
    pub fn new(use_gpu: bool, thread_limit: Arc<RwLock<Option<u32>>>) -> Self {
        Self { context: None, cached_state: Mutex::new(None), use_gpu, thread_limit }
    }
}

impl SttEngine for WhisperEngine {
    fn id(&self) -> &str {
        "whisper"
    }

    fn name(&self) -> &str {
        "Whisper"
    }

    fn load_model(&mut self, model_path: &Path) -> Result<(), AppError> {
        // Clear cached state from previous model
        if let Ok(mut guard) = self.cached_state.lock() {
            *guard = None;
        }

        let path_str = model_path
            .to_str()
            .ok_or_else(|| AppError::Stt("Invalid model path".to_string()))?;

        let mut ctx_params = WhisperContextParameters::default();
        if self.use_gpu {
            ctx_params.use_gpu(true);
            ctx_params.gpu_device(0);
            ctx_params.flash_attn(true);
        }

        let ctx = WhisperContext::new_with_params(path_str, ctx_params)
            .map_err(|e| AppError::Stt(format!("Failed to load Whisper model: {}", e)))?;

        self.context = Some(ctx);
        log::info!("Whisper model loaded: {}", model_path.display());
        Ok(())
    }

    fn unload_model(&mut self) {
        // Clear cached state before context
        if let Ok(mut guard) = self.cached_state.lock() {
            *guard = None;
        }
        self.context = None;
        log::info!("Whisper model unloaded");
    }

    fn is_loaded(&self) -> bool {
        self.context.is_some()
    }

    fn transcribe(&self, samples: &[f32], language: Option<&str>, initial_prompt: Option<&str>) -> Result<TranscriptionResult, AppError> {
        let ctx = self
            .context
            .as_ref()
            .ok_or_else(|| AppError::Stt("No model loaded".to_string()))?;

        let start = Instant::now();

        // Reuse cached state or create a new one.
        // Avoids re-allocating ~200MB of compute buffers per streaming segment.
        let mut state_guard = self.cached_state.lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let mut state = match state_guard.take() {
            Some(s) => s,
            None => ctx.create_state().map_err(|e| {
                AppError::Stt(format!("Failed to create whisper state: {}", e))
            })?,
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(language);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // CPU thread limit (live-updated via Arc<RwLock>)
        if let Some(n) = self.thread_limit.read().ok().and_then(|g| *g) {
            params.set_n_threads(n as i32);
        }

        // Additional optimizations (~10-15% CPU gain)
        params.set_suppress_blank(true);     // Skip blank/silence tokens
        params.set_suppress_nst(true);       // Skip non-speech tokens ([music], [applause]...)
        params.set_no_context(true);         // No cross-segment context (uses initial_prompt instead)
        params.set_temperature(0.0);         // Deterministic decoding

        // For short audio (< 30s at 16kHz), disable timestamps entirely.
        // Without timestamp tokens, whisper.cpp's "single timestamp ending -
        // skip entire chunk" heuristic can never trigger.  single_segment(true)
        // alone is NOT enough — the skip happens via `continue` before the
        // single_segment `break` is reached.
        if samples.len() < 480_000 {
            params.set_single_segment(true);
            params.set_no_timestamps(true);
            params.set_temperature_inc(0.0); // Disable fallback retries for streaming
        }

        // Provide previous transcription as decoder prompt context.
        // Helps Whisper maintain consistency across streaming segments
        // (e.g., "première phrase" context → more likely to produce "deuxième phrase").
        if let Some(prompt) = initial_prompt {
            if !prompt.is_empty() {
                params.set_initial_prompt(prompt);
            }
        }

        state.full(params, samples)?;

        // full_n_segments returns c_int directly (not Result)
        let num_segments = state.full_n_segments();
        let mut text = String::new();
        let mut segments = Vec::new();

        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let segment_text = segment
                    .to_str()
                    .map_err(|e| AppError::Stt(format!("Segment text error: {}", e)))?
                    .to_string();
                // Timestamps are in centiseconds (10ms units)
                let start_ms = (segment.start_timestamp() * 10) as u64;
                let end_ms = (segment.end_timestamp() * 10) as u64;

                text.push_str(&segment_text);
                segments.push(Segment {
                    text: segment_text,
                    start_ms,
                    end_ms,
                });
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        let detected_lang = {
            let lang_id = state.full_lang_id_from_state();
            get_lang_str(lang_id as i32).map(|s| s.to_string())
        };

        log::info!(
            "Transcription complete: {} segments, {}ms, lang={:?}",
            segments.len(),
            duration_ms,
            detected_lang
        );

        // Cache state for next call (avoids ~200MB re-allocation)
        *state_guard = Some(state);

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            language: detected_lang,
            segments,
            duration_ms,
        })
    }
}
