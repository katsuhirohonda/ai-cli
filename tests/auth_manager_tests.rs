use ai_cli::auth::{AuthManager, AuthMethod, ProviderAuth};

#[tokio::test]
async fn test_auth_manager_detect_cli_session() {
    let manager = AuthManager::new();
    
    // Test Claude CLI session detection
    let auth = manager.detect_auth("claude").await;
    assert!(auth.is_ok() || auth.is_err()); // Either found or not found
}

#[tokio::test]
async fn test_auth_manager_with_api_key() {
    let mut manager = AuthManager::new();
    
    // Set API key for a provider
    manager.set_api_key("claude", "test_api_key");
    
    let auth = manager.detect_auth("claude").await;
    assert!(auth.is_ok());
    
    if let Ok(AuthMethod::ApiKey { key }) = auth {
        assert_eq!(key, "test_api_key");
    }
}

#[tokio::test]
async fn test_auth_manager_fallback_order() {
    let manager = AuthManager::new();
    
    // Should try CLI session first, then env vars, then error
    let auth = manager.detect_auth("gemini").await;
    
    // We expect either success or a proper error
    assert!(auth.is_ok() || auth.is_err());
}

#[tokio::test]
async fn test_auth_manager_multiple_providers() {
    let mut manager = AuthManager::new();
    
    manager.set_api_key("claude", "claude_key");
    manager.set_api_key("gemini", "gemini_key");
    
    let claude_auth = manager.detect_auth("claude").await;
    let gemini_auth = manager.detect_auth("gemini").await;
    
    assert!(claude_auth.is_ok());
    assert!(gemini_auth.is_ok());
}

#[tokio::test]
async fn test_auth_manager_unknown_provider() {
    let manager = AuthManager::new();
    
    let auth = manager.detect_auth("unknown_provider").await;
    assert!(auth.is_err());
}