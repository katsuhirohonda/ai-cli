use ai_cli::auth::AuthManager;
use ai_cli::cli::{CliArgs, Command};
use ai_cli::pipeline::{PipelineExecutor, PipelineParser, PipelineStep};
use ai_cli::providers::{Context};
use ai_cli::providers::claude::ClaudeProvider;
use ai_cli::providers::gemini::GeminiProvider;
use ai_cli::providers::codex::CodexProvider;
use std::sync::Arc;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    // Basic verbosity handling (placeholder)
    let _verbose = args.verbose;

    // Auth manager and executor
    let auth = AuthManager::new();
    let mut executor = PipelineExecutor::new();

    // Register providers opportunistically via detected auth
    // Claude
    if let Ok(method) = auth.detect_auth("claude").await {
        match method {
            ai_cli::auth::AuthMethod::ApiKey { key } => {
                let prov = ClaudeProvider::new(key);
                executor.register_provider("claude", Arc::new(prov));
            }
            ai_cli::auth::AuthMethod::CliAuth => {
                if let Ok(prov) = ClaudeProvider::from_cli_session().await {
                    executor.register_provider("claude", Arc::new(prov));
                }
            }
            _ => {}
        }
    }
    // Gemini
    if let Ok(method) = auth.detect_auth("gemini").await {
        match method {
            ai_cli::auth::AuthMethod::ApiKey { key } => {
                let prov = GeminiProvider::new(key);
                executor.register_provider("gemini", Arc::new(prov));
            }
            ai_cli::auth::AuthMethod::CliAuth => {
                if let Ok(prov) = GeminiProvider::from_cli_session().await {
                    executor.register_provider("gemini", Arc::new(prov));
                }
            }
            _ => {}
        }
    }
    // Codex
    if let Ok(method) = auth.detect_auth("codex").await {
        match method {
            ai_cli::auth::AuthMethod::ApiKey { key } => {
                let prov = CodexProvider::new(key);
                executor.register_provider("codex", Arc::new(prov));
            }
            ai_cli::auth::AuthMethod::CliAuth => {
                if let Ok(prov) = CodexProvider::from_cli_session().await {
                    executor.register_provider("codex", Arc::new(prov));
                }
            }
            _ => {}
        }
    }

    // Parse command and dispatch
    match args.command {
        Some(Command::ListProviders) => {
            let names = executor.get_provider_names();
            if names.is_empty() {
                println!("No providers registered (auth not detected). Use --api-key on execute/pipeline.");
            } else {
                for n in names {
                    println!("{}", n);
                }
            }
        }
        Some(Command::CheckAuth { provider }) => {
            match auth.detect_auth(&provider).await {
                Ok(_) => println!("{}: authenticated or credentials detected", provider),
                Err(e) => println!("{}: auth not found ({})", provider, e),
            }
        }
        Some(Command::Version) => {
            println!("ai-cli version {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Command::Execute { provider, prompt, api_key, context, no_stream: _ }) => {
            // Ensure provider is registered; for now support only claude natively
            if !executor.has_provider(&provider) {
                if let Some(key) = api_key.clone() {
                    match provider.as_str() {
                        "claude" => executor.register_provider("claude", Arc::new(ClaudeProvider::new(key))),
                        "gemini" => executor.register_provider("gemini", Arc::new(GeminiProvider::new(key))),
                        "codex" => executor.register_provider("codex", Arc::new(CodexProvider::new(key))),
                        _ => {}
                    }
                }
            }

            if !executor.has_provider(&provider) {
                eprintln!("Provider '{}' not available. Use --api-key or configure auth.", provider);
                std::process::exit(1);
            }

            let mut ctx = Context::new();
            if let Some(path) = context {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    ctx.add_message(ai_cli::providers::Message::new(
                        ai_cli::providers::MessageRole::System,
                        format!("Context file {}:\n{}", path, text),
                    ));
                }
            }

            let steps = vec![PipelineStep::new(provider.clone(), prompt)];
            match executor.execute(&steps, ctx).await {
                Ok(responses) => {
                    for r in responses { println!("{}", r.content); }
                }
                Err(e) => {
                    eprintln!("Execution failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Command::Pipeline { chain, context, no_stream: _ }) => {
            // Parse pipeline chain
            let steps = match PipelineParser::parse(&chain) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Invalid chain: {}", e);
                    std::process::exit(1);
                }
            };

            // Validate against currently registered providers
            let names = executor.get_provider_names();
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            if let Err(e) = PipelineParser::validate_providers(&steps, &name_refs) {
                eprintln!("{}", e);
                eprintln!("Tip: provide API keys or login for missing providers.");
                std::process::exit(1);
            }

            let mut ctx = Context::new();
            if let Some(path) = context {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    ctx.add_message(ai_cli::providers::Message::new(
                        ai_cli::providers::MessageRole::System,
                        format!("Context file {}:\n{}", path, text),
                    ));
                }
            }

            match executor.execute(&steps, ctx).await {
                Ok(responses) => {
                    for (i, r) in responses.iter().enumerate() {
                        println!("[{}] {}", i + 1, r.content);
                    }
                }
                Err(e) => {
                    eprintln!("Pipeline failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // clap will show help by default due to arg_required_else_help
        }
    }
}
