use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use crate::audio::capture::AudioCapture;
use crate::config::schema::{AppConfig, HotkeyConfig};
use crate::history::store::HistoryStore;
use crate::injection::TextInjector;
use crate::llm::LlmBackend;
use crate::models::cache::ModelCache;
use crate::stt::SttEngine;

pub struct AppState {
    pub app_handle: tauri::AppHandle,
    pub config: Arc<RwLock<AppConfig>>,
    pub recording: Arc<RwLock<RecordingState>>,
    pub stt_engine: Arc<Mutex<Box<dyn SttEngine>>>,
    pub audio_capture: Arc<Mutex<AudioCapture>>,
    pub text_injector: Arc<dyn TextInjector>,
    pub history: Arc<Mutex<HistoryStore>>,
    pub is_recording: Arc<AtomicBool>,
    /// Stored as Option<Arc> so we can clone the Arc out of the RwLock
    /// before any .await (RwLockGuard is !Send).
    pub llm_backend: Arc<RwLock<Option<Arc<dyn LlmBackend>>>>,
    pub model_cache: Arc<Mutex<ModelCache>>,
    /// Shared with keyboard_hook thread — updated live when hotkey config changes.
    pub hotkey_config: Arc<RwLock<HotkeyConfig>>,
    /// Text processing hotkey — updated live. None = feature disabled.
    pub text_hotkey_config: Arc<RwLock<Option<HotkeyConfig>>>,
    /// Shared flag: true while simulating keystrokes (Ctrl+C/V via enigo).
    /// Prevents the keyboard hook from processing simulated events.
    pub is_simulating_keys: Arc<AtomicBool>,
    /// Cancel tokens for active model downloads. Key = model_id.
    pub active_downloads: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// Shared STT thread limit — updated live when user changes slider.
    pub stt_thread_limit: Arc<RwLock<Option<u32>>>,
    /// Handle of the streaming task (joined on stop).
    pub streaming_handle: Arc<Mutex<Option<
        tauri::async_runtime::JoinHandle<Result<crate::streaming::StreamingResult, crate::error::AppError>>
    >>>,
    /// Clipboard content saved at the start of streaming, restored at the end.
    pub saved_clipboard: Arc<Mutex<Option<String>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum RecordingState {
    Idle,
    Recording,
    Processing { stage: ProcessingStage },
}

#[derive(Debug, Clone, Serialize)]
pub enum ProcessingStage {
    Transcribing,
    PostProcessing,
    Injecting,
}
