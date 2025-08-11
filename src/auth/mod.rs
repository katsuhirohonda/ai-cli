use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AuthMethod {
    AccountBased {
        provider: String,
        session_token: Option<String>,
    },
    ApiKey {
        key: String,
    },
    BrowserAuth {
        callback_url: String,
    },
    CliAuth,
}

#[derive(Debug)]
pub struct ProviderAuth {
    pub provider: String,
    pub method: AuthMethod,
}

pub struct AuthManager {
    api_keys: HashMap<String, String>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            api_keys: HashMap::new(),
        }
    }

    pub fn set_api_key(&mut self, provider: &str, api_key: &str) {
        self.api_keys.insert(provider.to_string(), api_key.to_string());
    }

    pub async fn detect_auth(&self, provider: &str) -> Result<AuthMethod> {
        // 1. Check for API key in manager
        if let Some(api_key) = self.api_keys.get(provider) {
            return Ok(AuthMethod::ApiKey { key: api_key.clone() });
        }

        // 2. Check native CLI session
        if self.check_cli_session(provider).await? {
            return Ok(AuthMethod::CliAuth);
        }

        // 3. Check environment variables
        let env_var_name = format!("{}_API_KEY", provider.to_uppercase());
        if let Ok(key) = env::var(&env_var_name) {
            return Ok(AuthMethod::ApiKey { key });
        }

        // 4. No authentication found
        Err(anyhow!("No authentication found for provider: {}", provider))
    }

    async fn check_cli_session(&self, provider: &str) -> Result<bool> {
        let config_path = self.get_provider_config_path(provider)?;
        Ok(config_path.exists())
    }

    fn get_provider_config_path(&self, provider: &str) -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?;
        
        let config_path = match provider {
            "claude" => home.join(".claude").join("config.json"),
            "gemini" => home.join(".gemini").join("config.json"),
            "codex" => home.join(".codex").join("config.json"),
            _ => return Err(anyhow!("Unknown provider: {}", provider)),
        };
        
        Ok(config_path)
    }
}