use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub stt: SttConfig,
    pub postprocessing: PostProcessingConfig,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub hotkey: HotkeyConfig,
    pub input_device: Option<String>,
    pub auto_start: bool,
    pub auto_enter: bool,
    pub clipboard_restore: bool,
    /// STT language. None = auto-detect.
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub gpu_acceleration: bool,
    #[serde(default = "default_ui_language")]
    pub ui_language: String,
    /// Hotkey for text processing (select → reformulate/translate → replace).
    /// None = feature disabled.
    #[serde(default)]
    pub text_hotkey: Option<HotkeyConfig>,
}

fn default_ui_language() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub key: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    pub active_engine: String,
    pub active_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingConfig {
    pub auto_capitalize: bool,
    pub smart_spacing: bool,
    pub reformulation: ReformulationConfig,
    pub translation: TranslationConfig,
    pub substitutions: Vec<SubstitutionRule>,
    /// Custom prompts created by the user.
    #[serde(default)]
    pub custom_prompts: Vec<CustomPrompt>,
    /// Overrides for built-in prompts (key = style name, e.g. "Cleaned").
    #[serde(default)]
    pub prompt_overrides: HashMap<String, PromptOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReformulationConfig {
    pub enabled: bool,
    pub style: ReformulationStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReformulationStyle {
    Cleaned,
    Professional,
    Casual,
    Concise,
    Simplified,
    Structured,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPrompt {
    pub id: String,
    pub name: String,
    pub system: String,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOverride {
    pub system: Option<String>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub enabled: bool,
    pub target_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstitutionRule {
    pub from: String,
    pub to: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub active_backend: LlmBackendType,
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub local: LocalLlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalLlmConfig {
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LlmBackendType {
    Ollama,
    /// Local CPU-based LLM (llama.cpp). Alias for backwards compat with "LocalCandle".
    #[serde(alias = "LocalCandle")]
    Local,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                hotkey: HotkeyConfig {
                    key: "Space".to_string(),
                    modifiers: vec!["Control".to_string(), "Shift".to_string()],
                },
                input_device: None,
                auto_start: false,
                auto_enter: false,
                clipboard_restore: true,
                language: None,
                gpu_acceleration: false,
                ui_language: "en".to_string(),
                text_hotkey: None,
            },
            stt: SttConfig {
                active_engine: "whisper".to_string(),
                active_model: None,
            },
            postprocessing: PostProcessingConfig {
                auto_capitalize: true,
                smart_spacing: true,
                reformulation: ReformulationConfig {
                    enabled: false,
                    style: ReformulationStyle::Cleaned,
                },
                translation: TranslationConfig {
                    enabled: false,
                    target_language: "en".to_string(),
                },
                substitutions: Vec::new(),
                custom_prompts: Vec::new(),
                prompt_overrides: HashMap::new(),
            },
            llm: LlmConfig {
                active_backend: LlmBackendType::None,
                ollama: OllamaConfig {
                    host: "localhost".to_string(),
                    port: 11434,
                    model: "mistral".to_string(),
                },
                local: LocalLlmConfig::default(),
            },
        }
    }
}
