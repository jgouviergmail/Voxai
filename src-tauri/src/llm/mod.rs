pub mod ollama;
pub mod prompt_templates;
pub mod local_llm;

use async_trait::async_trait;

use crate::error::AppError;

/// Trait for LLM backends (Ollama, local llama.cpp, etc.)
#[async_trait]
pub trait LlmBackend: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn is_available(&self) -> bool;
    async fn generate(&self, prompt: &str, system: &str) -> Result<String, AppError>;
}
