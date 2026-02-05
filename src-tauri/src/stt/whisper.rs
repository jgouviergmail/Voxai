use std::path::Path;
use std::time::Instant;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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
}

impl WhisperEngine {
    pub fn new() -> Self {
        Self { context: None }
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
        let path_str = model_path
            .to_str()
            .ok_or_else(|| AppError::Stt("Invalid model path".to_string()))?;

        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
            .map_err(|e| AppError::Stt(format!("Failed to load Whisper model: {}", e)))?;

        self.context = Some(ctx);
        log::info!("Whisper model loaded: {}", model_path.display());
        Ok(())
    }

    fn unload_model(&mut self) {
        self.context = None;
        log::info!("Whisper model unloaded");
    }

    fn is_loaded(&self) -> bool {
        self.context.is_some()
    }

    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<TranscriptionResult, AppError> {
        let ctx = self
            .context
            .as_ref()
            .ok_or_else(|| AppError::Stt("No model loaded".to_string()))?;

        let start = Instant::now();

        // Create a temporary WhisperState for this transcription
        // (WhisperState is NOT Send+Sync, but WhisperContext IS)
        let mut state = ctx.create_state().map_err(|e| {
            AppError::Stt(format!("Failed to create whisper state: {}", e))
        })?;

        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        params.set_language(language);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

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

        log::info!(
            "Transcription complete: {} segments, {}ms",
            segments.len(),
            duration_ms
        );

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            language: None,
            segments,
            duration_ms,
        })
    }
}
