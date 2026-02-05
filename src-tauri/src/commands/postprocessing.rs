use std::sync::Arc;

use tauri::State;

use crate::app_state::AppState;
use crate::error::AppError;
use crate::llm::prompt_templates;
use crate::llm::LlmBackend;

#[tauri::command]
pub async fn check_llm_status(state: State<'_, AppState>) -> Result<LlmStatus, AppError> {
    // Clone the Arc out before any .await
    let backend: Option<Arc<dyn LlmBackend>> = {
        let guard = state
            .llm_backend
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        guard.clone()
    };

    match backend {
        Some(b) => {
            let available = b.is_available().await;
            Ok(LlmStatus {
                configured: true,
                available,
                backend_name: b.name().to_string(),
            })
        }
        None => Ok(LlmStatus {
            configured: false,
            available: false,
            backend_name: "None".to_string(),
        }),
    }
}

#[tauri::command]
pub async fn test_reformulation(
    text: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let style = {
        let config = state
            .config
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        config.postprocessing.reformulation.style.clone()
    };

    let backend: Arc<dyn LlmBackend> = {
        let guard = state
            .llm_backend
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        guard
            .clone()
            .ok_or_else(|| AppError::Llm("No LLM backend configured".to_string()))?
    };

    let prompt = prompt_templates::build_reformulation_prompt(&text, &style);
    backend.generate(&prompt.user, &prompt.system).await
}

#[tauri::command]
pub async fn test_translation(
    text: String,
    target_language: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let backend: Arc<dyn LlmBackend> = {
        let guard = state
            .llm_backend
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        guard
            .clone()
            .ok_or_else(|| AppError::Llm("No LLM backend configured".to_string()))?
    };

    let prompt = prompt_templates::build_translation_prompt(&text, &target_language);
    backend.generate(&prompt.user, &prompt.system).await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LlmStatus {
    pub configured: bool,
    pub available: bool,
    pub backend_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineTestResult {
    pub input: String,
    pub after_capitalize: String,
    pub after_spacing: String,
    pub after_reformulation: Option<String>,
    pub after_translation: Option<String>,
    pub after_substitutions: String,
    pub final_text: String,
}

#[tauri::command]
pub async fn test_text_pipeline(
    text: String,
    state: State<'_, AppState>,
) -> Result<PipelineTestResult, AppError> {
    use crate::postprocessing::{capitalization, spacing, substitutions};

    let pp_config = {
        let config = state
            .config
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        config.postprocessing.clone()
    };

    let backend: Option<Arc<dyn LlmBackend>> = {
        let guard = state
            .llm_backend
            .read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        guard.clone()
    };

    let input = text.clone();

    // 1. Capitalize
    let after_capitalize = if pp_config.auto_capitalize {
        capitalization::capitalize_sentences(&text)
    } else {
        text.clone()
    };

    // 2. Spacing
    let after_spacing = if pp_config.smart_spacing {
        spacing::normalize_spacing(&after_capitalize)
    } else {
        after_capitalize.clone()
    };

    // 3. Reformulation
    let mut current = after_spacing.clone();
    let after_reformulation = if pp_config.reformulation.enabled {
        if let Some(ref b) = backend {
            if b.is_available().await {
                let prompt = prompt_templates::build_reformulation_prompt(
                    &current,
                    &pp_config.reformulation.style,
                );
                match b.generate(&prompt.user, &prompt.system).await {
                    Ok(r) if !r.is_empty() => {
                        current = r.clone();
                        Some(r)
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 4. Translation
    let after_translation = if pp_config.translation.enabled {
        if let Some(ref b) = backend {
            if b.is_available().await {
                let prompt = prompt_templates::build_translation_prompt(
                    &current,
                    &pp_config.translation.target_language,
                );
                match b.generate(&prompt.user, &prompt.system).await {
                    Ok(t) if !t.is_empty() => {
                        current = t.clone();
                        Some(t)
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 5. Substitutions
    let after_substitutions = substitutions::apply_substitutions(&current, &pp_config.substitutions);

    Ok(PipelineTestResult {
        input,
        after_capitalize,
        after_spacing,
        after_reformulation,
        after_translation,
        after_substitutions: after_substitutions.clone(),
        final_text: after_substitutions,
    })
}
