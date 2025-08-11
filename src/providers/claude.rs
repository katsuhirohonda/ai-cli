use super::{AIProvider, Capabilities, Context, Response, ResponseStream};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use futures::stream;
use std::path::PathBuf;

/// Claude AI provider implementation
pub struct ClaudeProvider {
    api_key: Option<String>,
    is_cli_session: bool,
}

impl ClaudeProvider {
    /// Create a new Claude provider with an API key
    pub fn new(api_key: String) -> Self {
        Self { 
            api_key: Some(api_key),
            is_cli_session: false,
        }
    }

    /// Create a Claude provider from existing CLI session
    pub async fn from_cli_session() -> Result<Self> {
        // Check for Claude CLI session configuration
        let config_path = Self::get_claude_config_path()?;
        
        if config_path.exists() {
            // TODO: Parse actual Claude CLI config when format is known
            Ok(Self {
                api_key: None,
                is_cli_session: true,
            })
        } else {
            Err(anyhow!("No Claude CLI session found"))
        }
    }

    /// Create a provider assuming a detected CLI/session exists
    pub fn from_detected_cli_session() -> Self {
        Self { api_key: None, is_cli_session: true }
    }

    /// Get the path to Claude CLI configuration
    fn get_claude_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home.join(".claude").join("config.json"))
    }

    /// Check if provider is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.api_key.is_some() || self.is_cli_session
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    async fn execute(&self, prompt: &str, context: &Context) -> Result<Response> {
        if !self.is_authenticated() {
            return Err(anyhow!("Claude provider not authenticated"));
        }

        // TODO: Implement actual API call
        let response_text = format!("Claude response to: {}", prompt);
        
        let mut response = Response::new(response_text);
        if !context.conversation_history.is_empty() {
            response = response.with_metadata(
                "conversation_length", 
                context.conversation_history.len().to_string()
            );
        }
        
        Ok(response)
    }

    async fn stream(&self, prompt: &str, _context: &Context) -> Result<ResponseStream> {
        if !self.is_authenticated() {
            return Err(anyhow!("Claude provider not authenticated"));
        }

        // TODO: Implement actual streaming API call
        let response = format!("Claude streaming response to: {}", prompt);
        Ok(Box::pin(stream::once(async move { Ok(response) })))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_streaming: true,
            supports_context: true,
            max_tokens: 200000, // Claude 3's context window
        }
    }

    fn name(&self) -> &str {
        "claude"
    }
}
