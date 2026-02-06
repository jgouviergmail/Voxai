use async_trait::async_trait;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;

use super::LlmBackend;
use crate::error::AppError;

pub struct OllamaBackend {
    client: Ollama,
    model: String,
}

impl OllamaBackend {
    pub fn new(host: String, port: u16, model: String) -> Self {
        let client = Ollama::new(format!("http://{}", host), port);
        Self { client, model }
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    fn id(&self) -> &str {
        "ollama"
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    async fn is_available(&self) -> bool {
        self.client.list_local_models().await.is_ok()
    }

    async fn generate(&self, prompt: &str, system: &str) -> Result<String, AppError> {
        let request = GenerationRequest::new(self.model.clone(), prompt.to_string())
            .system(system.to_string());

        let response = self.client
            .generate(request)
            .await
            .map_err(|e| AppError::Llm(format!("Ollama generation failed: {}", e)))?;

        Ok(response.response.trim().to_string())
    }
}
