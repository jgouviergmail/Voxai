use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use tauri::Emitter;

use crate::audio::{resampler, silence::SilenceDetector};
use crate::config::schema::{AppConfig, PostProcessingConfig};
use crate::error::AppError;
use crate::events;
use crate::injection::TextInjector;
use crate::llm::LlmBackend;
use crate::postprocessing;
use crate::stt::{SttEngine, TranscriptionResult};

pub struct StreamingResult {
    pub raw_segments: Vec<String>,
    pub processed_segments: Vec<String>,
}

pub async fn run_streaming(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<f32>>,
    stt_engine: Arc<Mutex<Box<dyn SttEngine>>>,
    llm_backend: Arc<RwLock<Option<Arc<dyn LlmBackend>>>>,
    text_injector: Arc<dyn TextInjector>,
    config: Arc<RwLock<AppConfig>>,
    app: tauri::AppHandle,
    sample_rate: u32,
    channels: u16,
) -> Result<StreamingResult, AppError> {
    let mut detector = SilenceDetector::new(sample_rate, channels);
    let mut raw_segments: Vec<String> = Vec::new();
    let mut processed_segments: Vec<String> = Vec::new();
    let mut accumulated_display = String::new();
    let mut is_first_segment = true;
    // Previous transcription text used as prompt context for the next segment.
    let mut prev_transcription: Option<String> = None;

    // Resolve Silero VAD model path (bundled in binary, extracted on first use).
    // If unavailable, VAD validation is skipped (graceful degradation).
    let vad_model_path: Option<String> = match crate::stt::vad::ensure_vad_model() {
        Ok(path) => {
            let s = path.to_string_lossy().to_string();
            log::info!("[streaming] Silero VAD model ready: {}", s);
            Some(s)
        }
        Err(e) => {
            log::warn!("[streaming] Silero VAD unavailable (continuing without): {e}");
            None
        }
    };

    // Read config once at start (same pattern as batch lib.rs)
    let (stt_language, pp_cfg) = {
        let cfg = config
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let lang = match &cfg.general.language {
            Some(lang) if !lang.is_empty() => Some(lang.clone()),
            _ => None,
        };
        (lang, cfg.postprocessing.clone())
    };

    let mut chunk_count: u64 = 0;
    while let Some(chunk) = rx.recv().await {
        chunk_count += 1;
        if chunk_count % 500 == 0 {
            log::info!("[streaming] heartbeat: {} chunks received", chunk_count);
        }
        if let Some(speech_segment) = detector.process(&chunk) {
            log::info!(
                "[streaming] speech segment #{}: {} samples (~{:.1}s)",
                raw_segments.len() + 1,
                speech_segment.len(),
                speech_segment.len() as f64 / (sample_rate as f64 * channels as f64)
            );
            if let Err(e) = process_segment(
                speech_segment,
                channels,
                sample_rate,
                &stt_engine,
                &llm_backend,
                &text_injector,
                &stt_language,
                &pp_cfg,
                &app,
                &vad_model_path,
                &mut raw_segments,
                &mut processed_segments,
                &mut accumulated_display,
                &mut is_first_segment,
                &mut prev_transcription,
            )
            .await
            {
                log::warn!("[streaming] segment processing failed (continuing): {e}");
            }
        }
    }
    log::info!("[streaming] channel closed after {} chunks, {} segments", chunk_count, raw_segments.len());

    // Final segment (speech in progress without trailing silence)
    if let Some(speech_segment) = detector.flush() {
        if let Err(e) = process_segment(
            speech_segment,
            channels,
            sample_rate,
            &stt_engine,
            &llm_backend,
            &text_injector,
            &stt_language,
            &pp_cfg,
            &app,
            &vad_model_path,
            &mut raw_segments,
            &mut processed_segments,
            &mut accumulated_display,
            &mut is_first_segment,
            &mut prev_transcription,
        )
        .await
        {
            log::warn!("Streaming flush segment failed: {e}");
        }
    }

    Ok(StreamingResult {
        raw_segments,
        processed_segments,
    })
}

/// Process a single speech segment: resample -> VAD validate -> transcribe -> pipeline -> inject -> emit.
#[allow(clippy::too_many_arguments)]
async fn process_segment(
    speech_segment: Vec<f32>,
    channels: u16,
    sample_rate: u32,
    stt_engine: &Arc<Mutex<Box<dyn SttEngine>>>,
    llm_backend: &Arc<RwLock<Option<Arc<dyn LlmBackend>>>>,
    text_injector: &Arc<dyn TextInjector>,
    stt_language: &Option<String>,
    pp_cfg: &PostProcessingConfig,
    app: &tauri::AppHandle,
    vad_model_path: &Option<String>,
    raw_segments: &mut Vec<String>,
    processed_segments: &mut Vec<String>,
    accumulated_display: &mut String,
    is_first_segment: &mut bool,
    prev_transcription: &mut Option<String>,
) -> Result<(), AppError> {
    let segment_start = Instant::now();

    // 1. Resample (CPU-bound)
    let t0 = Instant::now();
    let samples_16k = tauri::async_runtime::spawn_blocking({
        let ch = channels;
        let sr = sample_rate;
        move || resampler::resample_to_16k_mono(&speech_segment, ch, sr)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;
    let resample_ms = t0.elapsed().as_millis();

    // 2+3. VAD validation + transcription in a single blocking task.
    // Merging avoids cloning samples_16k (~192KB) and an extra thread dispatch.
    let (vad_ms, transcribe_ms, transcription) = tauri::async_runtime::spawn_blocking({
        let stt = Arc::clone(stt_engine);
        let lang = stt_language.clone();
        let prompt = prev_transcription.clone();
        let vad_path = vad_model_path.clone();
        move || -> Result<(u128, u128, Option<TranscriptionResult>), AppError> {
            // VAD check first (no locks needed)
            let vad_start = Instant::now();
            if let Some(ref path) = vad_path {
                let has_speech = crate::stt::vad::validate_speech(path, &samples_16k)?;
                let vad_elapsed = vad_start.elapsed().as_millis();
                if !has_speech {
                    return Ok((vad_elapsed, 0u128, None));
                }
            }
            let vad_elapsed = vad_start.elapsed().as_millis();

            // Transcribe (acquires stt_engine lock)
            let transcribe_start = Instant::now();
            let engine = stt
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let result = engine.transcribe(&samples_16k, lang.as_deref(), prompt.as_deref())?;
            let transcribe_elapsed = transcribe_start.elapsed().as_millis();

            Ok((vad_elapsed, transcribe_elapsed, Some(result)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;

    let transcription = match transcription {
        Some(t) => t,
        None => {
            log::info!("[streaming] VAD rejected segment (no speech), skipping (vad={}ms)", vad_ms);
            return Ok(());
        }
    };

    if transcription.text.is_empty() {
        log::info!("[streaming] transcription empty, skipping (resample={}ms, vad={}ms, transcribe={}ms)",
            resample_ms, vad_ms, transcribe_ms);
        return Ok(());
    }

    raw_segments.push(transcription.text.clone());

    // Update prompt context for next segment (last ~200 chars to avoid token overflow).
    // Must find a valid UTF-8 char boundary — byte-level slicing panics on multi-byte chars.
    let prompt_text = &transcription.text;
    *prev_transcription = if prompt_text.len() > 200 {
        let mut start = prompt_text.len() - 200;
        while !prompt_text.is_char_boundary(start) {
            start += 1;
        }
        Some(prompt_text[start..].to_string())
    } else {
        Some(prompt_text.clone())
    };

    // 4. Full pipeline — reuses pipeline::run_pipeline() (DRY with batch)
    let t3 = Instant::now();
    let backend: Option<Arc<dyn LlmBackend>> = {
        let guard = llm_backend
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        guard.clone()
    };
    let source_lang = transcription
        .language
        .as_deref()
        .or(stt_language.as_deref());

    let segment_text = match postprocessing::pipeline::run_pipeline(
        &transcription.text,
        pp_cfg,
        backend.as_deref(),
        source_lang,
    )
    .await
    {
        Ok(text) => text,
        Err(e) => {
            log::warn!("Streaming pipeline failed for segment: {e}");
            transcription.text.clone()
        }
    };
    let pipeline_ms = t3.elapsed().as_millis();

    // 5. Add space before segment (except the first)
    let inject_text = if *is_first_segment {
        *is_first_segment = false;
        segment_text.clone()
    } else {
        format!(" {}", segment_text)
    };

    processed_segments.push(segment_text.clone());

    // 6. Inject at cursor (blocking — clipboard + paste)
    let t4 = Instant::now();
    let inj = Arc::clone(text_injector);
    let text_for_inject = inject_text;
    tauri::async_runtime::spawn_blocking(move || inj.inject_no_enter(&text_for_inject))
        .await
        .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;
    let inject_ms = t4.elapsed().as_millis();

    // 7. Emit accumulated text for overlay
    if !accumulated_display.is_empty() {
        accumulated_display.push(' ');
    }
    accumulated_display.push_str(&segment_text);
    let _ = app.emit(events::EVENT_TRANSCRIPTION_PARTIAL, &*accumulated_display);

    let total_ms = segment_start.elapsed().as_millis();
    log::info!(
        "[streaming] PERF segment #{}: total={}ms | resample={}ms vad={}ms transcribe={}ms pipeline={}ms inject={}ms | text={:?}",
        raw_segments.len(),
        total_ms,
        resample_ms,
        vad_ms,
        transcribe_ms,
        pipeline_ms,
        inject_ms,
        &transcription.text,
    );

    Ok(())
}
