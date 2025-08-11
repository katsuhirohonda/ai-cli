use anyhow::Result;
use async_trait::async_trait;
use crate::providers::Response;
use thiserror::Error;

/// Errors that can occur during transform operations
#[derive(Debug, Error)]
pub enum TransformError {
    #[error("JSON parsing failed: {0}")]
    JsonParse(#[from] serde_json::Error),
    
    #[error("Field '{field}' not found in JSON")]
    FieldNotFound { field: String },
    
    #[error("Transform operation failed: {0}")]
    Operation(String),
}

/// Behavior when JSON field extraction fails
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackBehavior {
    /// Keep the original content unchanged
    KeepOriginal,
    /// Return empty content
    ReturnEmpty,
    /// Return an error
    ReturnError,
}

impl Default for FallbackBehavior {
    fn default() -> Self {
        Self::KeepOriginal
    }
}

/// Configuration for JSON extractor transform
#[derive(Debug, Clone)]
pub struct JsonExtractorConfig {
    pub field: String,
    pub fallback_behavior: FallbackBehavior,
}

impl JsonExtractorConfig {
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            fallback_behavior: FallbackBehavior::default(),
        }
    }
    
    pub fn with_fallback(mut self, behavior: FallbackBehavior) -> Self {
        self.fallback_behavior = behavior;
        self
    }
}

/// Trait for transforming responses between pipeline steps
#[async_trait]
pub trait Transform: Send + Sync {
    /// Transform a response from one step to be used as input for the next step
    async fn transform(&self, response: Response) -> Result<Response>;
    
    /// Get the name of this transform
    fn name(&self) -> &str;
}

/// Identity transform that passes through responses unchanged
pub struct IdentityTransform;

#[async_trait]
impl Transform for IdentityTransform {
    async fn transform(&self, response: Response) -> Result<Response> {
        Ok(response)
    }
    
    fn name(&self) -> &str {
        "identity"
    }
}

/// JSON extractor transform that extracts JSON content from response
pub struct JsonExtractorTransform {
    config: JsonExtractorConfig,
}

impl JsonExtractorTransform {
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            config: JsonExtractorConfig::new(field),
        }
    }
    
    pub fn with_config(config: JsonExtractorConfig) -> Self {
        Self { config }
    }
    
    pub fn with_fallback(field: impl Into<String>, behavior: FallbackBehavior) -> Self {
        Self {
            config: JsonExtractorConfig::new(field).with_fallback(behavior),
        }
    }
}

#[async_trait]
impl Transform for JsonExtractorTransform {
    async fn transform(&self, mut response: Response) -> Result<Response> {
        // Parse JSON and extract field
        let json: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(TransformError::JsonParse)?;
        
        match json.get(&self.config.field) {
            Some(value) => {
                response.content = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
            }
            None => {
                match self.config.fallback_behavior {
                    FallbackBehavior::KeepOriginal => {
                        // Keep the original content - no change needed
                    }
                    FallbackBehavior::ReturnEmpty => {
                        response.content.clear();
                    }
                    FallbackBehavior::ReturnError => {
                        return Err(TransformError::FieldNotFound {
                            field: self.config.field.clone(),
                        }.into());
                    }
                }
            }
        }
        
        Ok(response)
    }
    
    fn name(&self) -> &str {
        "json_extractor"
    }
}

/// Summarizer transform that summarizes the response content
pub struct SummarizerTransform {
    pub max_length: usize,
}

impl SummarizerTransform {
    pub fn new(max_length: usize) -> Self {
        Self { max_length }
    }
}

#[async_trait]
impl Transform for SummarizerTransform {
    async fn transform(&self, mut response: Response) -> Result<Response> {
        // Truncate at Unicode character boundaries
        if response.content.chars().count() > self.max_length {
            response.content = response.content.chars()
                .take(self.max_length)
                .collect();
        }
        Ok(response)
    }
    
    fn name(&self) -> &str {
        "summarizer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_identity_transform() {
        let transform = IdentityTransform;
        let response = Response::new("test content")
            .with_metadata("key", "value");
        
        let result = transform.transform(response.clone()).await.unwrap();
        assert_eq!(result.content, response.content);
        assert_eq!(result.metadata, response.metadata);
    }

    #[tokio::test]
    async fn test_json_extractor_transform() {
        let transform = JsonExtractorTransform::new("data");
        let json_content = r#"{"data": "extracted value", "other": "ignored"}"#;
        let response = Response::new(json_content);
        
        let result = transform.transform(response).await.unwrap();
        assert_eq!(result.content, "extracted value");
    }

    #[tokio::test]
    async fn test_json_extractor_missing_field() {
        let transform = JsonExtractorTransform::new("missing_field");
        let json_content = r#"{"other": "value"}"#;
        let response = Response::new(json_content);
        
        let result = transform.transform(response).await.unwrap();
        // With default behavior (KeepOriginal), should return original content
        assert_eq!(result.content, json_content);
    }

    #[tokio::test]
    async fn test_json_extractor_missing_field_return_empty() {
        let transform = JsonExtractorTransform::with_fallback("missing_field", FallbackBehavior::ReturnEmpty);
        let json_content = r#"{"other": "value"}"#;
        let response = Response::new(json_content);
        
        let result = transform.transform(response).await.unwrap();
        assert_eq!(result.content, "");
    }

    #[tokio::test]
    async fn test_json_extractor_missing_field_return_error() {
        let transform = JsonExtractorTransform::with_fallback("missing_field", FallbackBehavior::ReturnError);
        let json_content = r#"{"other": "value"}"#;
        let response = Response::new(json_content);
        
        let result = transform.transform(response).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Field 'missing_field' not found"));
    }

    #[tokio::test]
    async fn test_json_extractor_invalid_json() {
        let transform = JsonExtractorTransform::new("field");
        let invalid_json = "not json content";
        let response = Response::new(invalid_json);
        
        let result = transform.transform(response).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("JSON parsing failed"));
    }

    #[tokio::test]
    async fn test_json_extractor_non_string_value() {
        let transform = JsonExtractorTransform::new("number");
        let json_content = r#"{"number": 42, "bool": true, "array": [1, 2, 3]}"#;
        let response = Response::new(json_content);
        
        let result = transform.transform(response).await.unwrap();
        assert_eq!(result.content, "42");
    }

    #[tokio::test]
    async fn test_summarizer_transform() {
        let transform = SummarizerTransform::new(10);
        let long_content = "This is a very long content that needs to be summarized";
        let response = Response::new(long_content);
        
        let result = transform.transform(response).await.unwrap();
        assert!(result.content.chars().count() <= 10);
        assert_eq!(result.content, "This is a ");
    }

    #[tokio::test]
    async fn test_summarizer_unicode_handling() {
        let transform = SummarizerTransform::new(5);
        let unicode_content = "これは日本語のテスト";
        let response = Response::new(unicode_content);
        
        let result = transform.transform(response).await.unwrap();
        assert!(result.content.chars().count() <= 5);
        assert_eq!(result.content, "これは日本");
    }

    #[tokio::test]
    async fn test_summarizer_short_content() {
        let transform = SummarizerTransform::new(100);
        let short_content = "Short";
        let response = Response::new(short_content);
        
        let result = transform.transform(response).await.unwrap();
        assert_eq!(result.content, "Short");
    }

    #[tokio::test]
    async fn test_json_extractor_config() {
        let config = JsonExtractorConfig::new("field")
            .with_fallback(FallbackBehavior::ReturnError);
        let transform = JsonExtractorTransform::with_config(config);
        
        assert_eq!(transform.name(), "json_extractor");
    }
}