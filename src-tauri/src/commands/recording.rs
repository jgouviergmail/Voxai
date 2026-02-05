use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{Emitter, State};

use crate::app_state::{AppState, ProcessingStage, RecordingState};
use crate::error::AppError;
use crate::events::EVENT_RECORDING_STATE_CHANGED;
use crate::tray::{update_tray_icon, TrayState};

#[tauri::command]
pub fn get_recording_state(state: State<'_, AppState>) -> Result<RecordingState, AppError> {
    let recording = state
        .recording
        .read()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(recording.clone())
}

#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
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
            .map_err(|e| AppError::Internal(e.to_string()))?;
        *recording = RecordingState::Recording;
    }

    let _ = update_tray_icon(&app, TrayState::Recording);
    let _ = app.emit(EVENT_RECORDING_STATE_CHANGED, RecordingState::Recording);

    let device_name = {
        let config = state
            .config
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        config.general.input_device.clone()
    };

    let mut capture = state
        .audio_capture
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Err(e) = capture.start(device_name.as_deref()) {
        drop(capture);
        crate::reset_state(&app, &state.is_recording, &state.recording);
        return Err(e);
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
    }

    let captured = {
        let mut capture = state
            .audio_capture
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        capture.stop()?
    };

    crate::set_processing_state(&app, &state.recording, ProcessingStage::Transcribing)?;
    let _ = update_tray_icon(&app, TrayState::Processing);

    let stt_engine = Arc::clone(&state.stt_engine);
    let text_injector = Arc::clone(&state.text_injector);
    let history = Arc::clone(&state.history);
    let config = Arc::clone(&state.config);
    let recording = Arc::clone(&state.recording);
    let is_recording = Arc::clone(&state.is_recording);
    let llm_backend = Arc::clone(&state.llm_backend);

    tauri::async_runtime::spawn(async move {
        let result = crate::run_pipeline(
            &app,
            captured,
            stt_engine,
            text_injector,
            history,
            config,
            llm_backend,
            &recording,
        )
        .await;

        if let Err(e) = &result {
            log::error!("Processing error: {}", e);
            let _ = app.emit(crate::events::EVENT_ERROR, format!("{}", e));
        }

        crate::reset_state(&app, &is_recording, &recording);
    });

    Ok(())
}

