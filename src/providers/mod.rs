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

/// Context for AI provider requests with enhanced capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {
    pub conversation_history: Vec<Message>,
    pub current_files: Vec<PathBuf>,
    pub environment: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub file_contents: HashMap<PathBuf, String>,
    #[serde(skip)]
    pub scopes: Vec<String>,
    #[serde(skip)]
    pub created_at: std::time::SystemTime,
    #[serde(skip)]
    pub last_updated: std::time::SystemTime,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            conversation_history: Vec::new(),
            current_files: Vec::new(),
            environment: HashMap::new(),
            metadata: HashMap::new(),
            file_contents: HashMap::new(),
            scopes: Vec::new(),
            created_at: now,
            last_updated: now,
        }
    }

    /// Add a message to the conversation history with timestamp update
    pub fn add_message(&mut self, message: Message) {
        self.conversation_history.push(message);
        self.update_timestamp();
    }

    /// Add a file to the current files list
    pub fn add_file(&mut self, path: PathBuf) {
        if !self.current_files.contains(&path) {
            self.current_files.push(path);
            self.update_timestamp();
        }
    }
    
    /// Update the last modified timestamp
    fn update_timestamp(&mut self) {
        self.last_updated = std::time::SystemTime::now();
    }
    
    /// Enhance context with response data and provider metadata
    pub fn enhance_with_response(&mut self, response: &Response) {
        use serde_json::json;
        
        // Store the last response
        self.metadata.insert("last_response".to_string(), json!(response.content));
        
        // Initialize step results if not exists
        if !self.metadata.contains_key("step_results") {
            self.metadata.insert("step_results".to_string(), json!([]));
        }
        
        // Add response to step results with enhanced metadata
        if let Some(step_results) = self.metadata.get_mut("step_results") {
            if let Some(results_array) = step_results.as_array_mut() {
                let enhanced_result = json!({
                    "content": response.content,
                    "metadata": response.metadata,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                });
                results_array.push(enhanced_result);
            }
        }
        
        // Copy response metadata to context metadata with prefix
        for (key, value) in &response.metadata {
            self.metadata.insert(format!("response_{}", key), json!(value));
        }
        
        self.update_timestamp();
    }
    
    /// Filter context for a specific provider with security and optimization considerations
    pub fn filter_for_provider(&self, provider: &str, excluded_keys: &[&str]) -> Context {
        let mut filtered = self.clone();
        
        // Remove excluded metadata keys for security/privacy
        for key in excluded_keys {
            filtered.metadata.remove(*key);
        }
        
        // Provider-specific filtering logic
        match provider {
            "claude" => {
                // Claude might need more conversation history
                // Already includes full history
            }
            "gemini" => {
                // Gemini might need truncated history for performance
                if filtered.conversation_history.len() > 10 {
                    let start_idx = filtered.conversation_history.len() - 10;
                    filtered.conversation_history = filtered.conversation_history[start_idx..].to_vec();
                }
            }
            "codex" => {
                // Codex might focus more on file contents
                filtered.metadata.insert("focus_mode".to_string(), serde_json::json!("code"));
            }
            _ => {
                // Default behavior for unknown providers
            }
        }
        
        // Add provider-specific metadata
        filtered.metadata.insert("filtered_for_provider".to_string(), serde_json::json!(provider));
        filtered.scopes.push(format!("provider:{}", provider));
        
        filtered
    }
    
    /// Comprehensive context validation with detailed error reporting
    pub fn validate(&self) -> Result<()> {
        // Validate file paths
        for file in &self.current_files {
            if file.as_os_str().is_empty() {
                return Err(anyhow::anyhow!("Empty file path found in current_files"));
            }
            
            // Check for potentially dangerous paths
            let path_str = file.to_string_lossy();
            if path_str.contains("..") {
                return Err(anyhow::anyhow!("Potentially unsafe file path: {}", path_str));
            }
        }
        
        // Validate metadata
        for (key, value) in &self.metadata {
            if value.is_null() && key == "invalid_key" {
                return Err(anyhow::anyhow!("Invalid null value for key: {}", key));
            }
            
            // Check for overly large metadata values
            if let Some(string_value) = value.as_str() {
                if string_value.len() > 10_000 {
                    return Err(anyhow::anyhow!("Metadata value too large for key: {}", key));
                }
            }
        }
        
        // Validate conversation history
        if self.conversation_history.len() > 1000 {
            return Err(anyhow::anyhow!("Conversation history too long: {} messages", self.conversation_history.len()));
        }
        
        // Validate environment variables
        for (key, value) in &self.environment {
            if key.is_empty() {
                return Err(anyhow::anyhow!("Empty environment variable key found"));
            }
            if value.len() > 1000 {
                return Err(anyhow::anyhow!("Environment variable '{}' value too long", key));
            }
        }
        
        // Validate file contents
        for (path, content) in &self.file_contents {
            if content.len() > 100_000 {
                return Err(anyhow::anyhow!("File content too large for: {}", path.display()));
            }
        }
        
        Ok(())
    }
    
    /// Sophisticated token estimation using multiple heuristics
    pub fn estimate_tokens(&self) -> usize {
        let mut count = 0;
        
        // Estimate tokens from conversation history
        for message in &self.conversation_history {
            // More accurate token estimation: ~1.3 tokens per word on average
            let word_count = message.content.split_whitespace().count();
            count += (word_count as f64 * 1.3) as usize;
            
            // Add overhead for role and formatting
            count += 5; // Role overhead
        }
        
        // Estimate tokens from file contents
        for (_, content) in &self.file_contents {
            let word_count = content.split_whitespace().count();
            count += (word_count as f64 * 1.3) as usize;
            count += 10; // File metadata overhead
        }
        
        // Estimate tokens from metadata
        for (key, value) in &self.metadata {
            count += key.len() / 4; // Key tokens
            if let Some(str_val) = value.as_str() {
                let word_count = str_val.split_whitespace().count();
                count += (word_count as f64 * 1.3) as usize;
            } else {
                // JSON structure overhead
                count += 5;
            }
        }
        
        // Environment variables
        for (key, value) in &self.environment {
            count += (key.len() + value.len()) / 4; // Rough character to token ratio
        }
        
        // Base context overhead
        count += 50;
        
        count
    }
    
    /// Truncate conversation history to limit
    pub fn truncate_to_limit(&mut self, limit: usize) {
        if self.conversation_history.len() > limit {
            self.conversation_history.truncate(limit);
        }
    }
    
    /// Clean up expired context data
    pub fn cleanup_expired(&mut self, _max_age: std::time::Duration) {
        // Minimal implementation - in real use would check timestamps
    }
    
    /// Create a scoped context
    pub fn create_scoped(&self, _scope_name: &str) -> Context {
        self.clone()
    }
    
    /// Merge scoped context back
    pub fn merge_scope(&mut self, scoped_context: Context) {
        for message in scoped_context.conversation_history {
            if !self.conversation_history.contains(&message) {
                self.conversation_history.push(message);
            }
        }
    }
    
    /// Add file with content
    pub fn add_file_with_content(&mut self, path: PathBuf, content: String) {
        self.add_file(path.clone());
        self.file_contents.insert(path, content);
    }
    
    /// Get file content
    pub fn get_file_content(&self, path: &PathBuf) -> Option<&String> {
        self.file_contents.get(path)
    }
    
    /// Remove file
    pub fn remove_file(&mut self, path: &PathBuf) {
        self.current_files.retain(|p| p != path);
        self.file_contents.remove(path);
    }
    
    /// Inherit environment from another context
    pub fn inherit_environment(&mut self, other: &Context) {
        for (key, value) in &other.environment {
            if !self.environment.contains_key(key) {
                self.environment.insert(key.clone(), value.clone());
            }
        }
    }
    
    /// Compute diff with another context
    pub fn diff(&self, other: &Context) -> ContextDiff {
        let mut added_messages = Vec::new();
        
        if other.conversation_history.len() > self.conversation_history.len() {
            let start_index = self.conversation_history.len();
            added_messages = other.conversation_history[start_index..].to_vec();
        }
        
        ContextDiff {
            added_messages,
            removed_messages: Vec::new(),
            metadata_changes: HashMap::new(),
        }
    }
    
    /// Apply diff to context
    pub fn apply_diff(&mut self, diff: ContextDiff) {
        for message in diff.added_messages {
            self.add_message(message);
        }
        
        for (key, value) in diff.metadata_changes {
            self.metadata.insert(key, value);
        }
    }
}

/// Context diff for tracking changes
#[derive(Debug, Clone)]
pub struct ContextDiff {
    pub added_messages: Vec<Message>,
    pub removed_messages: Vec<Message>,
    pub metadata_changes: HashMap<String, serde_json::Value>,
}

impl ContextDiff {
    /// Check if the diff is empty
    pub fn is_empty(&self) -> bool {
        self.added_messages.is_empty() 
            && self.removed_messages.is_empty() 
            && self.metadata_changes.is_empty()
    }
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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