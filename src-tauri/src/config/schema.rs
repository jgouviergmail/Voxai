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
    pub language: String,
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
                language: "fr".to_string(),
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
