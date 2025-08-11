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
        // 1. Prefer existing CLI/session credentials
        if self.check_cli_session(provider).await? {
            return Ok(AuthMethod::CliAuth);
        }

        // 2. Manager-provided API key (programmatic)
        if let Some(api_key) = self.api_keys.get(provider) {
            return Ok(AuthMethod::ApiKey { key: api_key.clone() });
        }

        // 3. Environment variables (provider-specific aliases first)
        if let Some(key) = self.get_env_api_key(provider) {
            return Ok(AuthMethod::ApiKey { key });
        }

        // 4. No authentication found
        Err(anyhow!("No authentication found for provider: {}", provider))
    }

    async fn check_cli_session(&self, provider: &str) -> Result<bool> {
        let candidates = self.get_cli_session_candidates(provider)?;
        Ok(candidates.into_iter().any(|p| p.exists()))
    }

    fn get_cli_session_candidates(&self, provider: &str) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?;

        match provider {
            // Known places where editors may store Claude session/state.
            // We only check for existence; we do NOT parse secrets here.
            "claude" => {
                // VS Code (macOS)
                paths.push(home.join("Library/Application Support/Code/User/globalStorage/anthropic.claude-copilot"));
                // VS Code (Linux)
                paths.push(home.join(".config/Code/User/globalStorage/anthropic.claude-copilot"));
                // VS Code (Windows)
                if let Ok(appdata) = env::var("APPDATA") {
                    paths.push(PathBuf::from(appdata).join("Code/User/globalStorage/anthropic.claude-copilot"));
                }
                // Legacy/placeholder path (backward compatibility)
                paths.push(home.join(".claude/config.json"));
            }
            // For Gemini, prefer gcloud ADC as a sign-in indicator
            "gemini" => {
                paths.push(home.join(".config/gcloud/application_default_credentials.json"));
                if let Ok(appdata) = env::var("APPDATA") {
                    paths.push(PathBuf::from(appdata).join("gcloud/application_default_credentials.json"));
                }
                paths.push(home.join(".gemini/config.json"));
            }
            // Unknown tooling; keep legacy path as a marker
            "codex" => {
                paths.push(home.join(".codex/config.json"));
            }
            _ => return Err(anyhow!("Unknown provider: {}", provider)),
        }

        Ok(paths)
    }

    fn get_env_api_key(&self, provider: &str) -> Option<String> {
        match provider {
            "claude" => {
                env::var("ANTHROPIC_API_KEY").ok()
                    .or_else(|| env::var("CLAUDE_API_KEY").ok())
            }
            "gemini" => {
                env::var("GEMINI_API_KEY").ok()
                    .or_else(|| env::var("GOOGLE_API_KEY").ok())
            }
            "codex" => env::var("CODEX_API_KEY").ok(),
            other => env::var(format!("{}_API_KEY", other.to_uppercase())).ok(),
        }
    }
}
