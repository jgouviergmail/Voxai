use tauri::State;

use crate::app_state::AppState;
use crate::config::persistence;
use crate::error::AppError;
use crate::models::registry;
use crate::stt::whisper::WHISPER_LANGUAGES;

#[tauri::command]
pub fn list_engines(state: State<'_, AppState>) -> Result<Vec<EngineInfo>, AppError> {
    let config = state
        .config
        .read()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let cache = state
        .model_cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let stt = state
        .stt_engine
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let downloaded = cache.list_downloaded();

    // STT models
    let stt_models: Vec<EngineModelInfo> = registry::MODEL_CATALOG
        .iter()
        .filter(|def| def.engine == "whisper")
        .map(|def| EngineModelInfo {
            id: def.id.to_string(),
            name: def.name.to_string(),
            size_mb: def.size_mb,
            description: def.description.to_string(),
            downloaded: downloaded.contains(&def.id.to_string()),
            active: config.stt.active_model.as_deref() == Some(def.id),
        })
        .collect();

    // LLM models
    let llm_models: Vec<EngineModelInfo> = registry::MODEL_CATALOG
        .iter()
        .filter(|def| def.engine == "llm")
        .map(|def| EngineModelInfo {
            id: def.id.to_string(),
            name: def.name.to_string(),
            size_mb: def.size_mb,
            description: def.description.to_string(),
            downloaded: downloaded.contains(&def.id.to_string()),
            active: config.llm.local.model_id.as_deref() == Some(def.id),
        })
        .collect();

    let llm_loaded = {
        let backend = state
            .llm_backend
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        backend.as_ref().map_or(false, |b| b.id() == "local")
    };

    let mut engines = vec![EngineInfo {
        id: stt.id().to_string(),
        name: stt.name().to_string(),
        active: config.stt.active_engine == stt.id(),
        loaded: stt.is_loaded(),
        models: stt_models,
    }];

    if !llm_models.is_empty() {
        engines.push(EngineInfo {
            id: "llm".to_string(),
            name: "Local LLM".to_string(),
            active: config.llm.active_backend == crate::config::schema::LlmBackendType::Local,
            loaded: llm_loaded,
            models: llm_models,
        });
    }

    Ok(engines)
}

#[tauri::command]
pub async fn set_active_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let model_def = registry::find_model(&model_id)
        .ok_or_else(|| AppError::Model(format!("Unknown model: {}", model_id)))?;

    let path = {
        let cache = state
            .model_cache
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        cache
            .model_path(&model_id)
            .ok_or_else(|| AppError::Model(format!("Model {} is not downloaded", model_id)))?
    };

    if model_def.engine == "llm" {
        // LLM model — update local config and rebuild backend
        {
            let mut config = state
                .config
                .write()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            config.llm.local.model_id = Some(model_id.clone());
            persistence::save_config(&config)?;
        }

        let new_backend = {
            let config = state
                .config
                .read()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let cache = state
                .model_cache
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            crate::build_local_llm_backend(&config, &cache)
        };

        let mut backend = state
            .llm_backend
            .write()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        *backend = new_backend;
    } else {
        // STT model
        {
            let mut engine = state
                .stt_engine
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            engine.load_model(&path)?;
        }

        {
            let mut config = state
                .config
                .write()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            config.stt.active_model = Some(model_id.clone());
            persistence::save_config(&config)?;
        }
    }

    log::info!("Active model set to {}", model_id);
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub loaded: bool,
    pub models: Vec<EngineModelInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u64,
    pub description: String,
    pub downloaded: bool,
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
}

/// Common languages shown at the top of language selectors.
const PRIORITY_LANGUAGES: &[&str] = &[
    "en", "fr", "es", "de", "it", "pt", "ja", "zh", "ru", "ar", "ko",
];

#[tauri::command]
pub fn list_supported_languages() -> Vec<LanguageInfo> {
    // Priority languages first, then rest alphabetically
    let mut priority: Vec<LanguageInfo> = Vec::new();
    let mut rest: Vec<LanguageInfo> = Vec::new();

    for &(code, name) in WHISPER_LANGUAGES {
        let info = LanguageInfo {
            code: code.to_string(),
            name: name.to_string(),
        };
        if PRIORITY_LANGUAGES.contains(&code) {
            priority.push(info);
        } else {
            rest.push(info);
        }
    }

    // Sort priority by the defined order
    priority.sort_by_key(|l| {
        PRIORITY_LANGUAGES
            .iter()
            .position(|&c| c == l.code)
            .unwrap_or(usize::MAX)
    });
    // Rest already alphabetical from WHISPER_LANGUAGES (sorted by name)

    priority.extend(rest);
    priority
}
