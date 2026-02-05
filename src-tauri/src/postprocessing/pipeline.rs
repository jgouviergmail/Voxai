use crate::config::schema::PostProcessingConfig;
use crate::error::AppError;
use crate::llm::prompt_templates;
use crate::llm::LlmBackend;

use super::{capitalization, spacing, substitutions};

/// Runs the full post-processing pipeline on raw transcription text.
/// Order: capitalize → spacing → reformulate → translate → substitute
/// Substitutions are always last so they're never overwritten by the LLM.
pub async fn run_pipeline(
    raw_text: &str,
    config: &PostProcessingConfig,
    llm: Option<&dyn LlmBackend>,
) -> Result<String, AppError> {
    let mut text = raw_text.to_string();

    // 1. Capitalization
    if config.auto_capitalize {
        text = capitalization::capitalize_sentences(&text);
    }

    // 2. Smart spacing
    if config.smart_spacing {
        text = spacing::normalize_spacing(&text);
    }

    // 3. Reformulation (requires LLM)
    if config.reformulation.enabled {
        if let Some(backend) = llm {
            if backend.is_available().await {
                let prompt =
                    prompt_templates::build_reformulation_prompt(&text, &config.reformulation.style);
                match backend.generate(&prompt.user, &prompt.system).await {
                    Ok(reformulated) => {
                        if !reformulated.is_empty() {
                            text = reformulated;
                        }
                    }
                    Err(e) => {
                        log::warn!("Reformulation failed, using original text: {}", e);
                        // Graceful degradation: keep text as-is
                    }
                }
            } else {
                log::warn!("LLM backend not available, skipping reformulation");
            }
        }
    }

    // 4. Translation (requires LLM)
    if config.translation.enabled {
        if let Some(backend) = llm {
            if backend.is_available().await {
                let prompt = prompt_templates::build_translation_prompt(
                    &text,
                    &config.translation.target_language,
                );
                match backend.generate(&prompt.user, &prompt.system).await {
                    Ok(translated) => {
                        if !translated.is_empty() {
                            text = translated;
                        }
                    }
                    Err(e) => {
                        log::warn!("Translation failed, using original text: {}", e);
                    }
                }
            } else {
                log::warn!("LLM backend not available, skipping translation");
            }
        }
    }

    // 5. Substitutions (LAST — never overwritten by LLM)
    text = substitutions::apply_substitutions(&text, &config.substitutions);

    Ok(text)
}
