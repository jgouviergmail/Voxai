use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use tauri::{Emitter, Manager};

mod app_state;
mod audio;
mod commands;
mod config;
mod error;
mod events;
mod history;
mod hotkey;
mod injection;
mod llm;
mod models;
mod postprocessing;
mod stt;
mod tray;

use app_state::{AppState, ProcessingStage, RecordingState};
use audio::capture::AudioCapture;
use config::persistence;
use config::schema::LlmBackendType;
use history::store::HistoryStore;
use injection::create_injector;
use llm::LlmBackend;
use models::cache::ModelCache;
use stt::whisper::WhisperEngine;

pub fn run() {
    env_logger::init();

    let mut app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Load config
            let config = persistence::load_config().unwrap_or_default();

            // Initialize history
            let history_path =
                history::store::history_path().expect("Failed to determine history path");
            let history_store =
                HistoryStore::new(history_path).expect("Failed to initialize history");

            // Initialize STT engine
            let stt_engine: Box<dyn stt::SttEngine> = Box::new(WhisperEngine::new());

            // Initialize text injector
            let text_injector = create_injector();

            // Initialize model cache
            let model_cache =
                ModelCache::new().expect("Failed to initialize model cache");

            // Initialize LLM backend based on config
            let llm_backend: Option<Arc<dyn LlmBackend>> = match &config.llm.active_backend {
                LlmBackendType::Ollama => {
                    let backend = llm::ollama::OllamaBackend::new(
                        config.llm.ollama.host.clone(),
                        config.llm.ollama.port,
                        config.llm.ollama.model.clone(),
                    );
                    Some(Arc::new(backend))
                }
                LlmBackendType::Local => {
                    build_local_llm_backend(&config, &model_cache)
                }
                LlmBackendType::None => None,
            };

            // Shared hotkey config — updated live when user changes settings
            let hotkey_config = Arc::new(RwLock::new(config.general.hotkey.clone()));

            // Build AppState
            let app_state = AppState {
                app_handle: app.handle().clone(),
                config: Arc::new(RwLock::new(config.clone())),
                recording: Arc::new(RwLock::new(RecordingState::Idle)),
                stt_engine: Arc::new(Mutex::new(stt_engine)),
                audio_capture: Arc::new(Mutex::new(AudioCapture::new())),
                text_injector: Arc::from(text_injector),
                history: Arc::new(Mutex::new(history_store)),
                is_recording: Arc::new(AtomicBool::new(false)),
                llm_backend: Arc::new(RwLock::new(llm_backend)),
                model_cache: Arc::new(Mutex::new(model_cache)),
                hotkey_config: Arc::clone(&hotkey_config),
                active_downloads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            };

            app.manage(app_state);

            // Build system tray
            let app_handle = app.handle().clone();
            tray::build_tray(&app_handle).expect("Failed to build system tray");

            // Start hotkey listener
            let hotkey_rx = hotkey::start_listener(hotkey_config);
            let app_handle_hotkey = app.handle().clone();

            std::thread::spawn(move || {
                while let Ok(event) = hotkey_rx.recv() {
                    let app = app_handle_hotkey.clone();
                    match event {
                        hotkey::HotkeyEvent::RecordStart => {
                            tauri::async_runtime::spawn(async move {
                                let state = app.state::<AppState>();
                                if let Err(e) = handle_record_start(&app, &state).await {
                                    log::error!("Failed to start recording: {}", e);
                                }
                            });
                        }
                        hotkey::HotkeyEvent::RecordStop => {
                            tauri::async_runtime::spawn(async move {
                                let state = app.state::<AppState>();
                                if let Err(e) = handle_record_stop(&app, &state).await {
                                    log::error!("Failed to stop recording: {}", e);
                                }
                            });
                        }
                    }
                }
            });

            // Auto-load model if available
            let app_handle_model = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle_model.state::<AppState>();

                let model_id = {
                    let config = state.config.read().ok();
                    config
                        .and_then(|c| c.stt.active_model.clone())
                        .unwrap_or_else(|| "whisper-base".to_string())
                };

                let path = {
                    let cache = match state.model_cache.lock() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Failed to lock model cache: {}", e);
                            return;
                        }
                    };
                    cache.model_path(&model_id)
                };

                if let Some(path) = path {
                    let mut engine = match state.stt_engine.lock() {
                        Ok(e) => e,
                        Err(e) => {
                            log::error!("Failed to lock STT engine: {}", e);
                            return;
                        }
                    };
                    if let Err(e) = engine.load_model(&path) {
                        log::error!("Failed to auto-load model: {}", e);
                    } else {
                        log::info!("Auto-loaded model: {}", model_id);
                    }
                } else {
                    log::info!("No model {} found, download required", model_id);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Recording
            commands::recording::get_recording_state,
            commands::recording::start_recording,
            commands::recording::stop_recording,
            // Settings
            commands::settings::get_settings,
            commands::settings::update_settings,
            // Audio
            commands::audio_devices::list_audio_devices,
            // History
            commands::history::get_history,
            commands::history::clear_history,
            // Models
            commands::models::list_models,
            commands::models::download_model,
            commands::models::delete_model,
            commands::models::cancel_download,
            // Engines
            commands::engines::list_engines,
            commands::engines::set_active_model,
            commands::engines::list_supported_languages,
            // Substitutions
            commands::substitutions::get_substitutions,
            commands::substitutions::add_substitution,
            commands::substitutions::delete_substitution,
            // LLM / Post-processing
            commands::postprocessing::check_llm_status,
            commands::postprocessing::test_reformulation,
            commands::postprocessing::test_translation,
            commands::postprocessing::test_text_pipeline,
            commands::postprocessing::get_prompt_preview,
            // GPU
            commands::gpu::detect_nvidia,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Must be called after build() — runtime is not available during setup()
    app.set_device_event_filter(tauri::DeviceEventFilter::Always);

    app.run(|app, event| {
            match event {
                tauri::RunEvent::WindowEvent { label, event, .. } => {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if label == "settings" {
                            // Close-to-tray: hide settings window, show overlay
                            api.prevent_close();
                            if let Some(window) = app.get_webview_window("settings") {
                                let _ = window.hide();
                            }
                            if let Some(overlay) = app.get_webview_window("overlay") {
                                let _ = overlay.show();
                            }
                        } else if label == "overlay" {
                            // Prevent overlay destruction (e.g. Alt+F4)
                            api.prevent_close();
                        }
                    }
                }
                tauri::RunEvent::ExitRequested { .. } => {
                    log::info!("Shutting down gracefully...");
                    let state = app.state::<AppState>();

                    // Stop recording if active
                    if state.is_recording.load(Ordering::SeqCst) {
                        state.is_recording.store(false, Ordering::SeqCst);
                        if let Ok(mut capture) = state.audio_capture.lock() {
                            let _ = capture.stop();
                        }
                    }

                    // Save config one last time
                    if let Ok(config) = state.config.read() {
                        let _ = persistence::save_config(&config);
                    }

                    log::info!("Shutdown complete");
                }
                _ => {}
            }
        });
}

/// Called when the push-to-talk key is pressed.
async fn handle_record_start(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<(), error::AppError> {
    // Atomic compare-exchange: only one caller can flip false→true
    if state
        .is_recording
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }

    {
        let mut recording = state
            .recording
            .write()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        *recording = RecordingState::Recording;
    }

    let _ = tray::update_tray_icon(app, tray::TrayState::Recording);
    let _ = app.emit(events::EVENT_RECORDING_STATE_CHANGED, RecordingState::Recording);

    let device_name = {
        let config = state
            .config
            .read()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        config.general.input_device.clone()
    };

    let mut capture = state
        .audio_capture
        .lock()
        .map_err(|e| error::AppError::Internal(e.to_string()))?;

    if let Err(e) = capture.start(device_name.as_deref()) {
        drop(capture);
        reset_state(app, &state.is_recording, &state.recording);
        return Err(e);
    }

    Ok(())
}

/// Called when the push-to-talk key is released.
async fn handle_record_stop(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<(), error::AppError> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
    }

    let captured = {
        let mut capture = state
            .audio_capture
            .lock()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        capture.stop()?
    };

    set_processing_state(app, &state.recording, ProcessingStage::Transcribing)?;
    let _ = tray::update_tray_icon(app, tray::TrayState::Processing);

    let stt_engine = Arc::clone(&state.stt_engine);
    let text_injector = Arc::clone(&state.text_injector);
    let history = Arc::clone(&state.history);
    let config_arc = Arc::clone(&state.config);
    let recording = Arc::clone(&state.recording);
    let is_recording = Arc::clone(&state.is_recording);
    let llm_backend = Arc::clone(&state.llm_backend);
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_pipeline(
            &app_clone,
            captured,
            stt_engine,
            text_injector,
            history,
            config_arc,
            llm_backend,
            &recording,
        )
        .await;

        if let Err(e) = result {
            log::error!("Pipeline error: {}", e);
            let _ = app_clone.emit(events::EVENT_ERROR, format!("{}", e));
        }

        reset_state(&app_clone, &is_recording, &recording);
    });

    Ok(())
}

/// Runs the full pipeline: resample → transcribe → post-process → inject → history.
pub(crate) async fn run_pipeline(
    app: &tauri::AppHandle,
    captured: audio::capture::CapturedAudio,
    stt_engine: Arc<Mutex<Box<dyn stt::SttEngine>>>,
    text_injector: Arc<dyn injection::TextInjector>,
    history: Arc<Mutex<HistoryStore>>,
    config: Arc<RwLock<config::schema::AppConfig>>,
    llm_backend: Arc<RwLock<Option<Arc<dyn LlmBackend>>>>,
    recording: &RwLock<RecordingState>,
) -> Result<(), error::AppError> {
    // 1. Resample to 16kHz mono
    let samples_16k = tauri::async_runtime::spawn_blocking({
        let samples = captured.samples;
        let channels = captured.channels;
        let sample_rate = captured.sample_rate;
        move || audio::resampler::resample_to_16k_mono(&samples, channels, sample_rate)
    })
    .await
    .map_err(|e| error::AppError::Internal(format!("Task join error: {}", e)))??;

    // 2. Transcribe (CPU-bound)
    let stt_language = {
        let cfg = config
            .read()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        let lang = cfg.general.language.clone();
        if lang.is_empty() { None } else { Some(lang) }
    };
    let stt_clone = Arc::clone(&stt_engine);
    let transcription = tauri::async_runtime::spawn_blocking(move || {
        let stt = stt_clone
            .lock()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        if !stt.is_loaded() {
            return Err(error::AppError::Stt("No STT model loaded".to_string()));
        }
        stt.transcribe(&samples_16k, stt_language.as_deref())
    })
    .await
    .map_err(|e| error::AppError::Internal(format!("Task join error: {}", e)))??;

    if transcription.text.is_empty() {
        log::info!("Empty transcription, skipping");
        return Ok(());
    }

    // 3. Post-process (with LLM if configured)
    set_processing_state(app, recording, ProcessingStage::PostProcessing)?;

    // Extract config and backend BEFORE the .await (guards must not cross await)
    let pp_config = {
        let cfg = config
            .read()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        cfg.postprocessing.clone()
    };

    let backend: Option<Arc<dyn LlmBackend>> = {
        let guard = llm_backend
            .read()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        guard.clone()
    };

    let final_text = postprocessing::pipeline::run_pipeline(
        &transcription.text,
        &pp_config,
        backend.as_deref(),
    )
    .await?;

    // 4. Inject
    set_processing_state(app, recording, ProcessingStage::Injecting)?;

    let options = {
        let cfg = config
            .read()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        injection::InjectionOptions {
            auto_enter: cfg.general.auto_enter,
            clipboard_restore: cfg.general.clipboard_restore,
        }
    };

    let injector_clone = Arc::clone(&text_injector);
    let text_clone = final_text.clone();
    tauri::async_runtime::spawn_blocking(move || injector_clone.inject(&text_clone, &options))
        .await
        .map_err(|e| error::AppError::Internal(format!("Task join error: {}", e)))??;

    // 5. History
    let entry = history::store::HistoryEntry::new(
        transcription.text.clone(),
        final_text,
        "whisper".to_string(),
        transcription.duration_ms,
    );

    {
        let mut hist = history
            .lock()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        hist.add(entry.clone())?;
    }

    let _ = app.emit(events::EVENT_TRANSCRIPTION_COMPLETE, &entry);

    Ok(())
}

pub(crate) fn set_processing_state(
    app: &tauri::AppHandle,
    recording: &RwLock<RecordingState>,
    stage: ProcessingStage,
) -> Result<(), error::AppError> {
    let new_state = RecordingState::Processing { stage };
    {
        let mut rec = recording
            .write()
            .map_err(|e| error::AppError::Internal(e.to_string()))?;
        *rec = new_state.clone();
    }
    let _ = app.emit(events::EVENT_RECORDING_STATE_CHANGED, new_state);
    Ok(())
}

pub(crate) fn reset_state(
    app: &tauri::AppHandle,
    is_recording: &AtomicBool,
    recording: &RwLock<RecordingState>,
) {
    is_recording.store(false, Ordering::SeqCst);
    if let Ok(mut rec) = recording.write() {
        *rec = RecordingState::Idle;
    }
    let _ = tray::update_tray_icon(app, tray::TrayState::Idle);
    let _ = app.emit(events::EVENT_RECORDING_STATE_CHANGED, RecordingState::Idle);
}

/// Build a local LLM backend from config + model cache.
/// Returns None if no model is configured or the model file is not downloaded.
pub(crate) fn build_local_llm_backend(
    config: &config::schema::AppConfig,
    model_cache: &models::cache::ModelCache,
) -> Option<Arc<dyn LlmBackend>> {
    let model_id = config.llm.local.model_id.as_deref().unwrap_or("phi-3.5-mini-q4");
    let model_path = match model_cache.model_path(model_id) {
        Some(p) => p,
        None => {
            log::warn!("Local LLM model '{}' not downloaded", model_id);
            return None;
        }
    };
    let model_def = models::registry::find_model(model_id);
    let model_name = model_def
        .map(|d| d.name.to_string())
        .unwrap_or_else(|| model_id.to_string());
    let chat_template = model_def.map(|d| d.chat_template).unwrap_or("phi3");
    let gpu_layers = if config.general.gpu_acceleration { Some(99) } else { None };

    match llm::local_llm::LocalLlmBackend::new(&model_path, model_name, chat_template, gpu_layers) {
        Ok(backend) => {
            log::info!("Local LLM backend loaded: {}", model_id);
            Some(Arc::new(backend))
        }
        Err(e) => {
            log::error!("Failed to load local LLM: {}", e);
            None
        }
    }
}
