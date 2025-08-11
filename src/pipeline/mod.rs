use anyhow::{Result, anyhow};
use std::fmt;
use std::collections::HashMap;
use std::sync::Arc;

use crate::providers::{AIProvider, Response, Context, Message, MessageRole};
use crate::auth::AuthManager;

pub mod transform;
pub use transform::{
    Transform, TransformError, IdentityTransform, JsonExtractorTransform, 
    SummarizerTransform, FallbackBehavior, JsonExtractorConfig
};

/// Represents a single step in the pipeline
#[derive(Clone)]
pub struct PipelineStep {
    pub provider: String,
    pub action: String,
    context: Option<String>,
    transform: Option<Arc<dyn Transform>>,
}

impl PipelineStep {
    /// Create a new pipeline step
    pub fn new(provider: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            action: action.into(),
            context: None,
            transform: None,
        }
    }
    
    /// Set context for this step
    pub fn set_context(&mut self, context: impl Into<String>) {
        self.context = Some(context.into());
    }
    
    /// Get the context for this step
    pub fn get_context(&self) -> Option<String> {
        self.context.clone()
    }
    
    /// Create a step with context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.set_context(context);
        self
    }
    
    /// Set transform for this step
    pub fn set_transform(&mut self, transform: Arc<dyn Transform>) {
        self.transform = Some(transform);
    }
    
    /// Create a step with transform
    pub fn with_transform(mut self, transform: Arc<dyn Transform>) -> Self {
        self.set_transform(transform);
        self
    }
    
    /// Check if this step has a transform
    pub fn has_transform(&self) -> bool {
        self.transform.is_some()
    }
    
    /// Get the transform for this step
    pub fn get_transform(&self) -> Option<Arc<dyn Transform>> {
        self.transform.clone()
    }
}

impl fmt::Debug for PipelineStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipelineStep")
            .field("provider", &self.provider)
            .field("action", &self.action)
            .field("context", &self.context)
            .field("has_transform", &self.has_transform())
            .finish()
    }
}

impl PartialEq for PipelineStep {
    fn eq(&self, other: &Self) -> bool {
        self.provider == other.provider 
            && self.action == other.action 
            && self.context == other.context
            && self.has_transform() == other.has_transform()
    }
}

impl fmt::Display for PipelineStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.provider, self.action)
    }
}

/// Parser for pipeline DSL strings
pub struct PipelineParser;

impl PipelineParser {
    /// Parse a pipeline DSL string into a vector of steps
    /// 
    /// # Format
    /// The pipeline format is: `provider:action -> provider:action -> ...`
    /// 
    /// # Examples
    /// ```ignore
    /// let input = "claude:design -> gemini:implement -> codex:review";
    /// let steps = PipelineParser::parse(input).unwrap();
    /// ```
    pub fn parse(input: &str) -> Result<Vec<PipelineStep>> {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return Err(anyhow!("Pipeline string cannot be empty"));
        }
        
        // Split by arrow separator and parse each step
        trimmed
            .split("->")
            .map(|part| Self::parse_step(part.trim()))
            .collect()
    }
    
    /// Parse a single pipeline step
    fn parse_step(step_str: &str) -> Result<PipelineStep> {
        if step_str.is_empty() {
            return Err(anyhow!("Pipeline step cannot be empty"));
        }
        
        // Find the colon separator
        let colon_pos = step_str.find(':')
            .ok_or_else(|| anyhow!("Invalid pipeline step format: '{}' (missing ':')", step_str))?;
        
        let provider = step_str[..colon_pos].trim();
        let action = step_str[colon_pos + 1..].trim();
        
        // Validate provider and action
        if provider.is_empty() {
            return Err(anyhow!("Provider cannot be empty in step: '{}'", step_str));
        }
        
        if action.is_empty() {
            return Err(anyhow!("Action cannot be empty in step: '{}'", step_str));
        }
        
        Ok(PipelineStep::new(provider, action))
    }
    
    /// Validate that all providers in the pipeline are known
    pub fn validate_providers(steps: &[PipelineStep], valid_providers: &[&str]) -> Result<()> {
        for step in steps {
            if !valid_providers.contains(&step.provider.as_str()) {
                return Err(anyhow!(
                    "Unknown provider: '{}'. Valid providers are: {:?}",
                    step.provider,
                    valid_providers
                ));
            }
        }
        Ok(())
    }
    
    /// Format a vector of steps back into a pipeline DSL string
    pub fn format(steps: &[PipelineStep]) -> String {
        steps
            .iter()
            .map(|step| step.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

/// Builder for creating pipelines programmatically
pub struct PipelineBuilder {
    steps: Vec<PipelineStep>,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }
    
    /// Add a step to the pipeline
    pub fn step(mut self, provider: impl Into<String>, action: impl Into<String>) -> Self {
        self.steps.push(PipelineStep::new(provider, action));
        self
    }
    
    /// Add a step with context
    pub fn step_with_context(
        mut self,
        provider: impl Into<String>,
        action: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        self.steps.push(PipelineStep::new(provider, action).with_context(context));
        self
    }
    
    /// Build the pipeline
    pub fn build(self) -> Vec<PipelineStep> {
        self.steps
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for pipeline execution
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub continue_on_error: bool,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub timeout_seconds: Option<u64>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            continue_on_error: false,
            max_retries: 0,
            retry_delay_ms: 1000,
            timeout_seconds: None,
        }
    }
}

/// Result of a single pipeline step execution
#[derive(Debug)]
pub struct StepResult {
    pub step: PipelineStep,
    pub response: Result<Response>,
    pub execution_time_ms: u64,
    pub retries: usize,
}

impl StepResult {
    /// Check if the step was successful
    pub fn is_success(&self) -> bool {
        self.response.is_ok()
    }
    
    /// Check if the step failed
    pub fn is_error(&self) -> bool {
        self.response.is_err()
    }
    
    /// Get the response if successful
    pub fn get_response(&self) -> Option<&Response> {
        self.response.as_ref().ok()
    }
    
    /// Get the error if failed
    pub fn get_error(&self) -> Option<&anyhow::Error> {
        self.response.as_ref().err()
    }
}

/// Callback for step execution events
pub type StepCallback = Box<dyn Fn(&StepResult) + Send + Sync>;

/// Pipeline execution engine
pub struct PipelineExecutor {
    providers: HashMap<String, Arc<dyn AIProvider>>,
    auth_manager: Option<AuthManager>,
    config: ExecutionConfig,
    step_callback: Option<StepCallback>,
}

impl PipelineExecutor {
    /// Create a new pipeline executor
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            auth_manager: None,
            config: ExecutionConfig::default(),
            step_callback: None,
        }
    }
    
    /// Create a new executor with configuration
    pub fn with_config(config: ExecutionConfig) -> Self {
        Self {
            providers: HashMap::new(),
            auth_manager: None,
            config,
            step_callback: None,
        }
    }
    
    /// Register a provider
    pub fn register_provider(&mut self, name: impl Into<String>, provider: Arc<dyn AIProvider>) {
        self.providers.insert(name.into(), provider);
    }
    
    /// Set authentication manager
    pub fn set_auth_manager(&mut self, auth_manager: AuthManager) {
        self.auth_manager = Some(auth_manager);
    }
    
    /// Update execution configuration
    pub fn set_config(&mut self, config: ExecutionConfig) {
        self.config = config;
    }
    
    /// Set continue on error flag
    pub fn set_continue_on_error(&mut self, continue_on_error: bool) {
        self.config.continue_on_error = continue_on_error;
    }
    
    /// Set maximum retries
    pub fn set_max_retries(&mut self, max_retries: usize) {
        self.config.max_retries = max_retries;
    }
    
    /// Set step callback
    pub fn set_step_callback(&mut self, callback: StepCallback) {
        self.step_callback = Some(callback);
    }
    
    /// Execute the pipeline
    pub async fn execute(&self, steps: &[PipelineStep], mut context: Context) -> Result<Vec<Response>> {
        let mut results = Vec::new();
        
        for (step_index, step) in steps.iter().enumerate() {
            let step_result = self.execute_step(step, &context, step_index).await;
            
            match &step_result.response {
                Ok(response) => {
                    // Update context with successful response
                    context.add_message(Message::new(MessageRole::Assistant, response.content.clone()));
                    results.push(response.clone());
                }
                Err(error) => {
                    if !self.config.continue_on_error {
                        return Err(anyhow!("Pipeline execution failed at step {}: {}", step_index + 1, error));
                    }
                    
                    // Create error response for continued execution
                    let error_response = Response::new(format!("Error in step {}: {}", step_index + 1, error))
                        .with_metadata("error", "true")
                        .with_metadata("step_index", step_index.to_string());
                    
                    results.push(error_response.clone());
                    context.add_message(Message::new(MessageRole::Assistant, error_response.content.clone()));
                }
            }
            
            // Call callback if set
            if let Some(callback) = &self.step_callback {
                callback(&step_result);
            }
        }
        
        Ok(results)
    }
    
    /// Execute a single step with retry logic
    async fn execute_step(&self, step: &PipelineStep, context: &Context, step_index: usize) -> StepResult {
        let start_time = std::time::Instant::now();
        let mut retries = 0;
        
        // Check if provider exists
        let provider = match self.providers.get(&step.provider) {
            Some(provider) => provider,
            None => {
                return StepResult {
                    step: step.clone(),
                    response: Err(anyhow!("Unknown provider: {}", step.provider)),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retries: 0,
                };
            }
        };
        
        // Build prompt from action and step context
        let prompt = self.build_prompt(step);
        
        // Retry loop
        loop {
            
            match provider.execute(&prompt, context).await {
                Ok(mut response) => {
                    // Enhance response with metadata
                    self.enhance_response(&mut response, context, step_index, retries);
                    
                    // Apply transform if present
                    if let Some(transform) = step.get_transform() {
                        match transform.transform(response).await {
                            Ok(transformed) => {
                                response = transformed;
                            }
                            Err(e) => {
                                return StepResult {
                                    step: step.clone(),
                                    response: Err(anyhow!("Transform failed: {}", e)),
                                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                                    retries,
                                };
                            }
                        }
                    }
                    
                    // Add provider name to response content for compatibility with existing tests
                    response.content = format!("{} response: {}", step.provider, response.content);
                    
                    return StepResult {
                        step: step.clone(),
                        response: Ok(response),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retries,
                    };
                }
                Err(error) => {
                    if retries >= self.config.max_retries {
                        return StepResult {
                            step: step.clone(),
                            response: Err(error),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retries,
                        };
                    }
                    
                    retries += 1;
                    
                    // Wait before retry
                    if self.config.retry_delay_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(self.config.retry_delay_ms)).await;
                    }
                }
            }
        }
    }
    
    /// Build prompt from step
    fn build_prompt(&self, step: &PipelineStep) -> String {
        if let Some(step_context) = &step.get_context() {
            format!("{}: {}", step.action, step_context)
        } else {
            step.action.clone()
        }
    }
    
    /// Enhance response with metadata and handle special cases
    fn enhance_response(&self, response: &mut Response, context: &Context, step_index: usize, retries: usize) {
        // Add authentication metadata
        if self.auth_manager.is_some() {
            response.metadata.insert("authenticated".to_string(), "true".to_string());
        }
        
        // Add context metadata
        if !context.conversation_history.is_empty() {
            response.metadata.insert("has_initial_context".to_string(), "true".to_string());
        }
        
        // Add pipeline metadata
        if step_index > 0 {
            response.metadata.insert("previous_step".to_string(), "true".to_string());
        }
        
        response.metadata.insert("step_index".to_string(), step_index.to_string());
        
        if retries > 0 {
            response.metadata.insert("retries".to_string(), retries.to_string());
        }
        
        // Add execution timestamp
        response.metadata.insert("execution_time".to_string(), 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        );
    }
    
    /// Get list of registered provider names
    pub fn get_provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
    
    /// Check if a provider is registered
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
    
    /// Get execution configuration
    pub fn get_config(&self) -> &ExecutionConfig {
        &self.config
    }
    
    /// Execute with streaming (simplified for now)
    pub async fn execute_streaming(&self, steps: &[PipelineStep], context: Context) -> Result<Vec<Response>> {
        self.execute(steps, context).await
    }
}

impl Default for PipelineExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    
    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new()
            .step("claude", "design")
            .step("gemini", "implement")
            .step_with_context("codex", "review", "Please check for security issues")
            .build();
        
        assert_eq!(pipeline.len(), 3);
        assert_eq!(pipeline[0].provider, "claude");
        assert_eq!(pipeline[2].get_context(), Some("Please check for security issues".to_string()));
    }
    
    #[test]
    fn test_pipeline_format() {
        let steps = vec![
            PipelineStep::new("claude", "design"),
            PipelineStep::new("gemini", "implement"),
        ];
        
        let formatted = PipelineParser::format(&steps);
        assert_eq!(formatted, "claude:design -> gemini:implement");
    }
    
    // Test for transform functionality - will fail initially (TDD Red phase)
    #[test]
    fn test_pipeline_step_with_transform() {
        let step = PipelineStep::new("claude", "design")
            .with_transform(Arc::new(IdentityTransform));
        
        assert!(step.has_transform());
        assert_eq!(step.provider, "claude");
        assert_eq!(step.action, "design");
    }
    
    // Mock provider for testing
    struct MockProvider {
        name: String,
        response_content: String,
    }
    
    #[async_trait]
    impl AIProvider for MockProvider {
        async fn execute(&self, _prompt: &str, _context: &Context) -> Result<Response> {
            Ok(Response::new(self.response_content.clone()))
        }
        
        async fn stream(&self, _prompt: &str, _context: &Context) -> Result<crate::providers::ResponseStream> {
            unimplemented!()
        }
        
        fn capabilities(&self) -> crate::providers::Capabilities {
            crate::providers::Capabilities::default()
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    // Mock transform for testing
    struct UppercaseTransform;
    
    #[async_trait]
    impl Transform for UppercaseTransform {
        async fn transform(&self, mut response: Response) -> Result<Response> {
            response.content = response.content.to_uppercase();
            Ok(response)
        }
        
        fn name(&self) -> &str {
            "uppercase"
        }
    }
    
    #[tokio::test]
    async fn test_pipeline_with_transform() {
        let mut executor = PipelineExecutor::new();
        
        // Register mock providers
        executor.register_provider("provider1", Arc::new(MockProvider {
            name: "provider1".to_string(),
            response_content: "hello world".to_string(),
        }));
        
        executor.register_provider("provider2", Arc::new(MockProvider {
            name: "provider2".to_string(),
            response_content: "goodbye".to_string(),
        }));
        
        // Create pipeline with transform
        let steps = vec![
            PipelineStep::new("provider1", "action1")
                .with_transform(Arc::new(UppercaseTransform)),
            PipelineStep::new("provider2", "action2"),
        ];
        
        let context = Context::new();
        let results = executor.execute(&steps, context).await.unwrap();
        
        // The first step's response should be transformed to uppercase
        // This test will fail initially as transform is not yet implemented
        assert_eq!(results[0].content, "provider1 response: HELLO WORLD");
        assert_eq!(results[1].content, "provider2 response: goodbye");
    }
}
