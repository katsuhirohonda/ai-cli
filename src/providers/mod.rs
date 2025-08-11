pub mod claude;

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use futures::stream::BoxStream;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Response from an AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub content: String,
    pub metadata: HashMap<String, String>,
}

impl Response {
    /// Create a new response with content
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Context for AI provider requests
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {
    pub conversation_history: Vec<Message>,
    pub current_files: Vec<PathBuf>,
    pub environment: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the conversation history
    pub fn add_message(&mut self, message: Message) {
        self.conversation_history.push(message);
    }

    /// Add a file to the current files list
    pub fn add_file(&mut self, path: PathBuf) {
        self.current_files.push(path);
    }
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    /// Create a new message
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Capabilities of an AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub supports_streaming: bool,
    pub supports_context: bool,
    pub max_tokens: usize,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            supports_streaming: false,
            supports_context: false,
            max_tokens: 4096,
        }
    }
}

/// Stream of response chunks
pub type ResponseStream<'a> = BoxStream<'a, Result<String>>;

/// Trait for AI providers
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Execute a prompt and return a response
    async fn execute(&self, prompt: &str, context: &Context) -> Result<Response>;
    
    /// Stream a response for the given prompt
    async fn stream(&self, prompt: &str, context: &Context) -> Result<ResponseStream>;
    
    /// Get the capabilities of this provider
    fn capabilities(&self) -> Capabilities;
    
    /// Get the name of this provider
    fn name(&self) -> &str;
}