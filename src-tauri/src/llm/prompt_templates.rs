use std::collections::HashMap;
use crate::config::schema::{CustomPrompt, PromptOverride, ReformulationStyle};

pub struct Prompt {
    pub system: String,
    pub user: String,
}

/// Built-in prompt defaults (style_description, instruction) for each style.
/// Style descriptions focus ONLY on what makes each style unique.
/// Shared constraints (language, person, output, profanity) are added by build_reformulation_prompt.
fn builtin_prompt(style: &str) -> Option<(&'static str, &'static str)> {
    match style {
        // Cleaned: full grammar reconstruction for spoken/dictated text.
        // Must fix broken sentence structures, not just typos.
        "Cleaned" => Some((
            "You are a grammar correction assistant for dictated text.\n\
             Task: rewrite the text with perfect grammar and sentence structure.\n\
             Rules:\n\
             - Fix ALL grammar, spelling, punctuation, and syntax errors.\n\
             - Reconstruct malformed or spoken-style sentences into proper written form.\n\
             - Example: \"C'est qui qui est venu hier?\" -> \"Qui est venu hier ?\"\n\
             - Example: \"me and him we went\" -> \"He and I went\"\n\
             - Example: \"the thing that I told you about it\" -> \"the thing I told you about\"\n\
             - Keep the same tone (formal stays formal, casual stays casual).\n\
             - Keep the same vocabulary where grammar allows it.\n\
             - Do NOT change the meaning, add new ideas, or remove information.",
            "Correct all grammar and sentence structure errors in this text:",
        )),
        "Professional" => Some((
            "You are a professional writing assistant.\n\
             Task: rewrite the text in a formal, business-appropriate tone.\n\
             Rules:\n\
             - Use polished vocabulary, proper grammar, and formal phrasing.\n\
             - Replace slang, casual expressions, and colloquialisms with professional equivalents.\n\
             - The result must read like a business email or official document.\n\
             - Fix any grammar or spelling errors along the way.\n\
             - Do NOT change the meaning, add new ideas, or remove information.",
            "Reformulate this text in a formal, professional tone:",
        )),
        "Casual" => Some((
            "You are a friendly writing assistant.\n\
             Task: rewrite the text in a relaxed, conversational tone.\n\
             Rules:\n\
             - Use everyday words, contractions, and a warm friendly voice.\n\
             - Replace stiff or formal phrasing with natural spoken language.\n\
             - The result must sound like talking to a friend.\n\
             - Fix any grammar or spelling errors along the way.\n\
             - Do NOT change the meaning, add new ideas, or remove information.",
            "Reformulate this text in a casual, friendly tone:",
        )),
        "Concise" => Some((
            "You are a concise writing assistant.\n\
             Task: shorten the text while keeping all key information.\n\
             Rules:\n\
             - Remove filler words, redundancy, and unnecessary detail.\n\
             - Merge short sentences when possible. Be direct and brief.\n\
             - The result must be significantly shorter than the input.\n\
             - Fix any grammar or spelling errors along the way.\n\
             - Do NOT remove important facts or change the meaning.",
            "Make this text shorter and more direct:",
        )),
        "Simplified" => Some((
            "You are a plain language assistant.\n\
             Task: simplify the text so anyone can understand it.\n\
             Rules:\n\
             - Use short sentences and simple common words.\n\
             - Replace jargon and complex terms with easy alternatives.\n\
             - Break long sentences into shorter ones.\n\
             - The result must be understandable by a child.\n\
             - Do NOT change the meaning, add new ideas, or remove information.",
            "Simplify this text:",
        )),
        "Structured" => Some((
            "You are a structured writing assistant.\n\
             Task: reorganize the text into clear paragraphs or bullet points.\n\
             Rules:\n\
             - Group related ideas together.\n\
             - Use bullet points (- ) for lists of items.\n\
             - Add clear paragraph breaks between topics.\n\
             - The result must be visually organized and easy to scan.\n\
             - Fix any grammar or spelling errors along the way.\n\
             - Do NOT change the meaning, add new ideas, or remove information.",
            "Restructure this text for clarity:",
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
    source_language: Option<&str>,
) -> Prompt {
    let (style_system, instruction) = resolve_prompt(style, custom_prompts, overrides);

    // === SANDWICH STRUCTURE ===
    // TOP: language constraint (primacy effect — small LLMs attend most to first lines)
    let lang_name = source_language
        .and_then(|c| crate::stt::whisper::language_name_from_code(c))
        .or(source_language);

    let lang_top = match lang_name {
        Some(name) => format!(
            "LANGUAGE: You MUST write in {} only. Never use another language.\n\n",
            name
        ),
        None => "LANGUAGE: Write in the SAME language as the input. Never switch language.\n\n"
            .to_string(),
    };

    // MIDDLE: style-specific description (from builtin or user override)

    // BOTTOM: critical constraints (recency effect — small LLMs also attend to last lines)
    let lang_bottom = match lang_name {
        Some(name) => format!("- LANGUAGE: Your output MUST be in {}.", name),
        None => "- LANGUAGE: Output in the same language as the input.".to_string(),
    };

    let constraints = format!(
        "\n\n\
         CRITICAL RULES (you MUST follow ALL of these):\n\
         1. PERSON: Keep the exact same grammatical person. \
         \"I/je\" stays \"I/je\". \"you/tu/vous\" stays \"you/tu/vous\". \
         NEVER switch to third person or impersonal form.\n\
         2. MEANING: Keep the exact same meaning, facts, and intent. Do not add or remove information.\n\
         3. PROFANITY: Replace any swear words, insults, vulgarities, or offensive language \
         with neutral, polite alternatives that preserve the intended meaning.\n\
         4. OUTPUT: Print ONLY the reformulated text. No explanation, no preamble, no label, no quotes, no guillemets.\n\
         5. {}",
        lang_bottom
    );

    let system = format!("{}{}{}", lang_top, style_system, constraints);

    // Guillemets to clearly delineate input text from instruction
    let user = format!("{}\n\n\u{ab}{}\u{bb}", instruction, text);

    Prompt { system, user }
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
             Preserve the original tone and meaning. Output ONLY the translated text, \
             nothing else — no explanations, no preamble, no quotation marks.",
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
            None,
        );
        // Sandwich: language at top
        assert!(prompt.system.starts_with("LANGUAGE:"));
        // Style in middle — grammar correction with sentence reconstruction
        assert!(prompt.system.contains("grammar correction"));
        assert!(prompt.system.contains("Reconstruct malformed"));
        // Constraints at bottom
        assert!(prompt.system.contains("PERSON:"));
        assert!(prompt.system.contains("PROFANITY:"));
        // User has guillemets
        assert!(prompt.user.contains("\u{ab}hello world\u{bb}"));
    }

    #[test]
    fn test_reformulation_prompt_professional() {
        let prompt = build_reformulation_prompt(
            "hey what's up",
            &ReformulationStyle::Professional,
            &[],
            &HashMap::new(),
            None,
        );
        assert!(prompt.system.contains("formal"));
        assert!(prompt.system.contains("business"));
        assert!(prompt.system.contains("PERSON:"));
        assert!(prompt.system.contains("PROFANITY:"));
    }

    #[test]
    fn test_reformulation_prompt_with_language() {
        let prompt = build_reformulation_prompt(
            "bonjour le monde",
            &ReformulationStyle::Cleaned,
            &[],
            &HashMap::new(),
            Some("fr"),
        );
        // Language at TOP (primacy)
        assert!(prompt.system.starts_with("LANGUAGE: You MUST write in French"));
        // Language at BOTTOM (recency)
        assert!(prompt.system.contains("output MUST be in French"));
    }

    #[test]
    fn test_critical_rules_in_all_styles() {
        for style in &[
            ReformulationStyle::Cleaned,
            ReformulationStyle::Professional,
            ReformulationStyle::Casual,
            ReformulationStyle::Concise,
            ReformulationStyle::Simplified,
            ReformulationStyle::Structured,
        ] {
            let prompt = build_reformulation_prompt(
                "test", style, &[], &HashMap::new(), None,
            );
            assert!(
                prompt.system.contains("PERSON:"),
                "Style {:?} missing person preservation", style
            );
            assert!(
                prompt.system.contains("PROFANITY:"),
                "Style {:?} missing profanity filter", style
            );
            assert!(
                prompt.system.contains("MEANING:"),
                "Style {:?} missing meaning preservation", style
            );
        }
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
            None,
        );
        assert!(prompt.user.contains("Make it rhyme"));
    }
}
