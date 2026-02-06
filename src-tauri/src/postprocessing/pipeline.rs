use std::time::Duration;

use crate::config::schema::PostProcessingConfig;
use crate::error::AppError;
use crate::llm::prompt_templates;
use crate::llm::LlmBackend;

use super::{capitalization, spacing, substitutions};

const LLM_TIMEOUT: Duration = Duration::from_secs(60);

/// Strip a matching pair of wrapping characters (e.g. «», "", "").
fn strip_wrapping(text: &str, open: char, close: char) -> String {
    let t = text.trim();
    if t.starts_with(open) && t.ends_with(close) && t.len() > open.len_utf8() + close.len_utf8() {
        t[open.len_utf8()..t.len() - close.len_utf8()].trim().to_string()
    } else if t.starts_with(open) {
        t[open.len_utf8()..].trim().to_string()
    } else {
        t.to_string()
    }
}

/// Strip `<think>...</think>` blocks and common LLM preambles.
fn strip_llm_artifacts(text: &str) -> String {
    let mut result = text.to_string();

    // 1. Strip <think>...</think> blocks (Qwen3 thinking mode)
    while let Some(start) = result.find("<think>") {
        if let Some(end) = result.find("</think>") {
            let end = end + "</think>".len();
            result = format!("{}{}", &result[..start], &result[end..]);
        } else {
            // Unclosed <think> — remove from <think> to end
            result = result[..start].to_string();
            break;
        }
    }

    let result = result.trim();

    // 2. Strip wrapping quotes / guillemets that small LLMs may echo back
    let result = strip_wrapping(result, '\u{ab}', '\u{bb}');
    let result = strip_wrapping(&result, '"', '"');
    let result = strip_wrapping(&result, '\u{201c}', '\u{201d}'); // " "
    let result = result.trim();

    // 3. Strip colon+newline preambles like "Here is the corrected text:\n"
    let limit = result.len().min(100);
    if let Some(pos) = result[..limit].find(":\n") {
        let after = result[pos + 2..].trim_start();
        if !after.is_empty() {
            return after.to_string();
        }
    }

    result.to_string()
}

/// Wraps an LLM generate call with a timeout to prevent indefinite hangs.
async fn llm_with_timeout(
    backend: &dyn LlmBackend,
    prompt: &str,
    system: &str,
) -> Result<String, AppError> {
    tokio::time::timeout(LLM_TIMEOUT, backend.generate(prompt, system))
        .await
        .map_err(|_| AppError::Llm("LLM generation timed out (60s)".into()))?
}

/// Runs the full post-processing pipeline on raw transcription text.
/// Order: capitalize → spacing → reformulate → translate → substitute
/// Substitutions are always last so they're never overwritten by the LLM.
pub async fn run_pipeline(
    raw_text: &str,
    config: &PostProcessingConfig,
    llm: Option<&dyn LlmBackend>,
    source_language: Option<&str>,
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
                let prompt = prompt_templates::build_reformulation_prompt(
                    &text,
                    &config.reformulation.style,
                    &config.custom_prompts,
                    &config.prompt_overrides,
                    source_language,
                );
                match llm_with_timeout(backend, &prompt.user, &prompt.system).await {
                    Ok(reformulated) => {
                        let cleaned = strip_llm_artifacts(&reformulated);
                        if !cleaned.is_empty() {
                            text = cleaned;
                        }
                    }
                    Err(e) => {
                        log::warn!("Reformulation failed, using original text: {}", e);
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
                match llm_with_timeout(backend, &prompt.user, &prompt.system).await {
                    Ok(translated) => {
                        let cleaned = strip_llm_artifacts(&translated);
                        if !cleaned.is_empty() {
                            text = cleaned;
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- strip_llm_artifacts: preamble stripping ---

    #[test]
    fn test_strip_preamble_with_prefix() {
        assert_eq!(
            strip_llm_artifacts("Here is the corrected text:\nHello world."),
            "Hello world."
        );
    }

    #[test]
    fn test_strip_preamble_with_sure_prefix() {
        assert_eq!(
            strip_llm_artifacts("Sure, here you go:\nBonjour le monde."),
            "Bonjour le monde."
        );
    }

    #[test]
    fn test_strip_preamble_with_extra_whitespace() {
        assert_eq!(
            strip_llm_artifacts("Here is the result:\n\n  Hello world."),
            "Hello world."
        );
    }

    #[test]
    fn test_strip_preamble_no_prefix() {
        assert_eq!(strip_llm_artifacts("Hello world."), "Hello world.");
    }

    #[test]
    fn test_strip_preamble_empty_after_colon() {
        // After trimming, "Here:\n" becomes "Here:" — no :\n within first 100 chars
        // so no stripping happens; result is trimmed "Here:"
        assert_eq!(strip_llm_artifacts("Here:\n"), "Here:");
    }

    #[test]
    fn test_strip_preamble_colon_deep_in_text() {
        let text = format!("{}:\nShould not strip this.", "A".repeat(100));
        assert_eq!(strip_llm_artifacts(&text), text);
    }

    #[test]
    fn test_strip_preamble_colon_without_newline() {
        assert_eq!(
            strip_llm_artifacts("Note: this has a colon but no newline"),
            "Note: this has a colon but no newline"
        );
    }

    // --- strip_llm_artifacts: <think> tag stripping ---

    #[test]
    fn test_strip_think_tags() {
        assert_eq!(
            strip_llm_artifacts("<think>Let me think about this...</think>Hello world."),
            "Hello world."
        );
    }

    #[test]
    fn test_strip_think_tags_unclosed() {
        assert_eq!(
            strip_llm_artifacts("Hello<think>some reasoning"),
            "Hello"
        );
    }

    #[test]
    fn test_strip_think_tags_with_preamble() {
        assert_eq!(
            strip_llm_artifacts("<think>reasoning</think>Here is the result:\nBonjour."),
            "Bonjour."
        );
    }

    #[test]
    fn test_strip_think_tags_multiple() {
        assert_eq!(
            strip_llm_artifacts("<think>first</think>Hello <think>second</think>world."),
            "Hello world."
        );
    }

    // --- strip_llm_artifacts: guillemet stripping ---

    #[test]
    fn test_strip_guillemets_wrapping() {
        assert_eq!(
            strip_llm_artifacts("\u{ab}Bonjour le monde.\u{bb}"),
            "Bonjour le monde."
        );
    }

    #[test]
    fn test_strip_guillemets_with_spaces() {
        assert_eq!(
            strip_llm_artifacts(" \u{ab}Hello world.\u{bb} "),
            "Hello world."
        );
    }

    #[test]
    fn test_strip_guillemets_only_opening() {
        // Only opening guillemet — still strip it
        assert_eq!(
            strip_llm_artifacts("\u{ab}Hello world."),
            "Hello world."
        );
    }

    #[test]
    fn test_no_guillemets_passthrough() {
        assert_eq!(
            strip_llm_artifacts("Hello world."),
            "Hello world."
        );
    }

    // --- strip_llm_artifacts: double-quote stripping ---

    #[test]
    fn test_strip_double_quotes_wrapping() {
        assert_eq!(
            strip_llm_artifacts("\"Bonjour le monde.\""),
            "Bonjour le monde."
        );
    }

    #[test]
    fn test_strip_curly_quotes_wrapping() {
        assert_eq!(
            strip_llm_artifacts("\u{201c}Hello world.\u{201d}"),
            "Hello world."
        );
    }

    #[test]
    fn test_preserve_internal_quotes() {
        // Quotes inside the text should NOT be stripped
        assert_eq!(
            strip_llm_artifacts("He said \"hello\" to me."),
            "He said \"hello\" to me."
        );
    }
}
