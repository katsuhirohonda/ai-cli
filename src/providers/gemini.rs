use super::{AIProvider, Capabilities, Context, Response, ResponseStream};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use futures::stream;
use std::path::PathBuf;

pub struct GeminiProvider {
    api_key: Option<String>,
    is_cli_session: bool,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key: Some(api_key), is_cli_session: false }
    }

    pub async fn from_cli_session() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        if config_path.exists() {
            Ok(Self { api_key: None, is_cli_session: true })
        } else {
            Err(anyhow!("No Gemini CLI session found"))
        }
    }

    /// Create a provider assuming a detected CLI/session exists
    pub fn from_detected_cli_session() -> Self {
        Self { api_key: None, is_cli_session: true }
    }

    fn get_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home.join(".gemini").join("config.json"))
    }

    fn is_authenticated(&self) -> bool { self.api_key.is_some() || self.is_cli_session }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn execute(&self, prompt: &str, context: &Context) -> Result<Response> {
        if !self.is_authenticated() { return Err(anyhow!("Gemini provider not authenticated")); }
        let response_text = format!("Gemini response to: {}", prompt);
        let mut response = Response::new(response_text);
        if !context.conversation_history.is_empty() {
            response = response.with_metadata("conversation_length", context.conversation_history.len().to_string());
        }
        Ok(response)
    }

    async fn stream(&self, prompt: &str, _context: &Context) -> Result<ResponseStream> {
        if !self.is_authenticated() { return Err(anyhow!("Gemini provider not authenticated")); }
        let response = format!("Gemini streaming response to: {}", prompt);
        Ok(Box::pin(stream::once(async move { Ok(response) })))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { supports_streaming: true, supports_context: true, max_tokens: 100000 }
    }

    fn name(&self) -> &str { "gemini" }
}
