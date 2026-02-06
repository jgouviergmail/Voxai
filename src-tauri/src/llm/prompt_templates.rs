use std::collections::HashMap;
use crate::config::schema::{CustomPrompt, PromptOverride, ReformulationStyle};

pub struct Prompt {
    pub system: String,
    pub user: String,
}

/// Built-in prompt defaults (system, instruction) for each style.
fn builtin_prompt(style: &str) -> Option<(&'static str, &'static str)> {
    match style {
        "Cleaned" => Some((
            "You are a text cleanup assistant. Fix grammar, punctuation, and minor errors while preserving the original meaning and tone. Do not add or remove information.",
            "Clean up the following dictated text. Fix any grammar or punctuation errors, but keep the original meaning intact:",
        )),
        "Professional" => Some((
            "You are a professional writing assistant. Reformulate text into clear, formal, business-appropriate language.",
            "Reformulate the following text in a professional, formal tone suitable for business communication:",
        )),
        "Casual" => Some((
            "You are a friendly writing assistant. Reformulate text into natural, conversational language.",
            "Reformulate the following text in a casual, friendly tone:",
        )),
        "Concise" => Some((
            "You are a concise writing assistant. Shorten text while preserving all key information. Remove filler words and redundancy.",
            "Make the following text more concise while keeping all important information:",
        )),
        "Simplified" => Some((
            "You are a plain language assistant. Simplify text to be easily understood by everyone. Use short sentences and common words.",
            "Simplify the following text for easy reading:",
        )),
        "Structured" => Some((
            "You are a structured writing assistant. Organize text into clear paragraphs or bullet points when appropriate.",
            "Restructure the following text for clarity, using paragraphs or bullet points if appropriate:",
        )),
        _ => None,
    }
}

/// Resolves the effective (system, instruction) for a style, applying overrides if present.
pub fn resolve_prompt(
    style: &ReformulationStyle,
    custom_prompts: &[CustomPrompt],
    overrides: &HashMap<String, PromptOverride>,
) -> (String, String) {
    match style {
        ReformulationStyle::Custom(id) => {
            if let Some(cp) = custom_prompts.iter().find(|p| p.id == *id) {
                (cp.system.clone(), cp.instruction.clone())
            } else {
                (
                    "You are a text reformulation assistant. Follow the user's instructions precisely.".to_string(),
                    id.clone(),
                )
            }
        }
        _ => {
            let style_name = format!("{:?}", style);
            let (default_sys, default_instr) = builtin_prompt(&style_name)
                .unwrap_or(("You are a helpful assistant.", "Reformulate:"));
            if let Some(ov) = overrides.get(&style_name) {
                let sys = ov.system.as_deref().unwrap_or(default_sys);
                let instr = ov.instruction.as_deref().unwrap_or(default_instr);
                (sys.to_string(), instr.to_string())
            } else {
                (default_sys.to_string(), default_instr.to_string())
            }
        }
    }
}

pub fn build_reformulation_prompt(
    text: &str,
    style: &ReformulationStyle,
    custom_prompts: &[CustomPrompt],
    overrides: &HashMap<String, PromptOverride>,
) -> Prompt {
    let (system, instruction) = resolve_prompt(style, custom_prompts, overrides);
    Prompt {
        system,
        user: format!("{}\n\n{}", instruction, text),
    }
}

/// Maps ISO 639-1 language codes to full language names for clearer LLM prompts.
/// Uses the shared WHISPER_LANGUAGES table so all 57 languages are covered.
fn language_name(code: &str) -> &str {
    crate::stt::whisper::language_name_from_code(code).unwrap_or(code)
}

pub fn build_translation_prompt(text: &str, target_language: &str) -> Prompt {
    let lang = language_name(target_language);
    Prompt {
        system: format!(
            "You are a professional translator. Translate the given text to {}. \
             Preserve the original tone and meaning. Output ONLY the translated text, nothing else.",
            lang
        ),
        user: text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reformulation_prompt_cleaned() {
        let prompt = build_reformulation_prompt(
            "hello world",
            &ReformulationStyle::Cleaned,
            &[],
            &HashMap::new(),
        );
        assert!(prompt.system.contains("cleanup"));
        assert!(prompt.user.contains("hello world"));
    }

    #[test]
    fn test_translation_prompt() {
        let prompt = build_translation_prompt("Bonjour le monde", "en");
        assert!(prompt.system.contains("English"));
        assert_eq!(prompt.user, "Bonjour le monde");
    }

    #[test]
    fn test_translation_prompt_korean() {
        let prompt = build_translation_prompt("Hello", "ko");
        assert!(prompt.system.contains("Korean"));
    }

    #[test]
    fn test_translation_prompt_unknown_code() {
        let prompt = build_translation_prompt("Hello", "xx");
        // Unknown code passes through as-is
        assert!(prompt.system.contains("xx"));
    }

    #[test]
    fn test_custom_style() {
        let prompt = build_reformulation_prompt(
            "test",
            &ReformulationStyle::Custom("Make it rhyme".to_string()),
            &[],
            &HashMap::new(),
        );
        assert!(prompt.user.contains("Make it rhyme"));
    }
}
