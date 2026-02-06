use std::sync::Arc;

use tauri::State;

use crate::app_state::AppState;
use crate::config::persistence;
use crate::config::schema::{AppConfig, LlmBackendType};
use crate::error::AppError;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, AppError> {
    let config = state
        .config
        .read()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(config.clone())
}

#[tauri::command]
pub fn update_settings(
    config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    persistence::save_and_notify(&config, &state.app_handle)?;

    // Check if LLM backend config changed — rebuild if needed
    let needs_llm_rebuild = {
        let current = state
            .config
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        current.llm.active_backend != config.llm.active_backend
            || current.llm.ollama.host != config.llm.ollama.host
            || current.llm.ollama.port != config.llm.ollama.port
            || current.llm.ollama.model != config.llm.ollama.model
            || current.llm.local.model_id != config.llm.local.model_id
            || current.general.gpu_acceleration != config.general.gpu_acceleration
    };

    // Update config
    {
        let mut current = state
            .config
            .write()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        *current = config.clone();
    }

    // Rebuild LLM backend if config changed
    if needs_llm_rebuild {
        let new_backend = match &config.llm.active_backend {
            LlmBackendType::Ollama => {
                let backend = crate::llm::ollama::OllamaBackend::new(
                    config.llm.ollama.host.clone(),
                    config.llm.ollama.port,
                    config.llm.ollama.model.clone(),
                );
                log::info!("LLM backend rebuilt: Ollama ({}:{})", config.llm.ollama.host, config.llm.ollama.port);
                Some(Arc::new(backend) as Arc<dyn crate::llm::LlmBackend>)
            }
            LlmBackendType::Local => {
                let cache = state
                    .model_cache
                    .lock()
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                crate::build_local_llm_backend(&config, &cache)
            }
            LlmBackendType::None => {
                log::info!("LLM backend disabled");
                None
            }
        };

        let mut backend = state
            .llm_backend
            .write()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        *backend = new_backend;
    }

    // Update shared hotkey config (live-read by keyboard_hook thread)
    {
        let mut hk = state
            .hotkey_config
            .write()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        *hk = config.general.hotkey.clone();
    }

    log::info!("Settings updated");
    Ok(())
}
