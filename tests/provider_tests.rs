use ai_cli::providers::{AIProvider, Capabilities, Context, Response, ResponseStream};
use async_trait::async_trait;
use futures::stream;
use anyhow::Result;

/// Mock provider for testing
struct MockProvider;

impl MockProvider {
    fn new() -> Self {
        MockProvider
    }
}

#[async_trait]
impl AIProvider for MockProvider {
    async fn execute(&self, _prompt: &str, _context: &Context) -> Result<Response> {
        Ok(Response::new("test response"))
    }

    async fn stream(&self, _prompt: &str, _context: &Context) -> Result<ResponseStream> {
        Ok(Box::pin(stream::once(async { Ok("test".to_string()) })))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_streaming: true,
            supports_context: true,
            max_tokens: 4096,
        }
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[tokio::test]
async fn test_provider_execute() {
    let provider = MockProvider::new();
    let context = Context::default();
    
    let response = provider.execute("test prompt", &context).await;
    
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.content, "test response");
}

#[tokio::test]
async fn test_provider_name() {
    let provider = MockProvider::new();
    
    assert_eq!(provider.name(), "mock");
}

#[tokio::test]
async fn test_provider_capabilities() {
    let provider = MockProvider::new();
    
    let capabilities = provider.capabilities();
    assert!(capabilities.supports_streaming);
    assert!(capabilities.supports_context);
}

#[tokio::test]
async fn test_provider_stream() {
    let provider = MockProvider::new();
    let context = Context::default();
    
    let stream = provider.stream("test prompt", &context).await;
    
    assert!(stream.is_ok());
}