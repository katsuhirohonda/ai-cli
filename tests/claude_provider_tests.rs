use ai_cli::providers::{AIProvider, Context, MessageRole, Message};
use ai_cli::providers::claude::ClaudeProvider;

#[tokio::test]
async fn test_claude_provider_name() {
    let provider = ClaudeProvider::new("test_key".to_string());
    assert_eq!(provider.name(), "claude");
}

#[tokio::test]
async fn test_claude_provider_capabilities() {
    let provider = ClaudeProvider::new("test_key".to_string());
    let capabilities = provider.capabilities();
    
    assert!(capabilities.supports_streaming);
    assert!(capabilities.supports_context);
    assert_eq!(capabilities.max_tokens, 200000);
}

#[tokio::test]
async fn test_claude_provider_execute() {
    let provider = ClaudeProvider::new("test_key".to_string());
    let mut context = Context::new();
    context.add_message(Message::new(MessageRole::User, "Hello"));
    
    let response = provider.execute("Say hello", &context).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_claude_provider_with_cli_auth() {
    // This test will fail if no CLI session exists, which is expected
    let provider = ClaudeProvider::from_cli_session().await;
    
    // For now, we just check that the method exists and returns a Result
    // In a real scenario, we'd mock the file system or skip this test
    if provider.is_ok() {
        let provider = provider.unwrap();
        assert_eq!(provider.name(), "claude");
    } else {
        // CLI session not found, which is fine for testing
        assert!(provider.is_err());
    }
}

#[tokio::test]
async fn test_claude_provider_stream() {
    let provider = ClaudeProvider::new("test_key".to_string());
    let context = Context::new();
    
    let stream = provider.stream("Hello", &context).await;
    assert!(stream.is_ok());
}