use crate::config::schema::ReformulationStyle;

pub struct Prompt {
    pub system: String,
    pub user: String,
}

pub fn build_reformulation_prompt(text: &str, style: &ReformulationStyle) -> Prompt {
    let (system, instruction) = match style {
        ReformulationStyle::Cleaned => (
            "You are a text cleanup assistant. Fix grammar, punctuation, and minor errors while preserving the original meaning and tone. Do not add or remove information.",
            "Clean up the following dictated text. Fix any grammar or punctuation errors, but keep the original meaning intact:",
        ),
        ReformulationStyle::Professional => (
            "You are a professional writing assistant. Reformulate text into clear, formal, business-appropriate language.",
            "Reformulate the following text in a professional, formal tone suitable for business communication:",
        ),
        ReformulationStyle::Casual => (
            "You are a friendly writing assistant. Reformulate text into natural, conversational language.",
            "Reformulate the following text in a casual, friendly tone:",
        ),
        ReformulationStyle::Concise => (
            "You are a concise writing assistant. Shorten text while preserving all key information. Remove filler words and redundancy.",
            "Make the following text more concise while keeping all important information:",
        ),
        ReformulationStyle::Simplified => (
            "You are a plain language assistant. Simplify text to be easily understood by everyone. Use short sentences and common words.",
            "Simplify the following text for easy reading:",
        ),
        ReformulationStyle::Structured => (
            "You are a structured writing assistant. Organize text into clear paragraphs or bullet points when appropriate.",
            "Restructure the following text for clarity, using paragraphs or bullet points if appropriate:",
        ),
        ReformulationStyle::Custom(instruction) => (
            "You are a text reformulation assistant. Follow the user's instructions precisely.",
            instruction.as_str(),
        ),
    };

    Prompt {
        system: system.to_string(),
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
        let prompt = build_reformulation_prompt("hello world", &ReformulationStyle::Cleaned);
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
        );
        assert!(prompt.user.contains("Make it rhyme"));
    }
}
