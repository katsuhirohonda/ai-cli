use anyhow::{Result, anyhow};
use std::fmt;

/// Represents a single step in the pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStep {
    pub provider: String,
    pub action: String,
    context: Option<String>,
}

impl PipelineStep {
    /// Create a new pipeline step
    pub fn new(provider: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            action: action.into(),
            context: None,
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
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    
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
}