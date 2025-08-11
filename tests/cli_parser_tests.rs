use ai_cli::cli::{CliArgs, Command};

#[test]
fn test_parse_basic_execute_command() {
    let args = vec!["ai-cli", "--provider", "claude", "--prompt", "Hello, world!"];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Execute { provider, prompt, api_key: _, context: _, no_stream }) => {
            assert_eq!(provider, "claude");
            assert_eq!(prompt, "Hello, world!");
            assert!(!no_stream); // stream is true by default
        }
        _ => panic!("Expected Execute command"),
    }
}

#[test]
fn test_parse_execute_with_api_key() {
    let args = vec![
        "ai-cli",
        "--provider", "gemini",
        "--prompt", "Test prompt",
        "--api-key", "test-key-123"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Execute { provider, prompt, api_key, context: _, no_stream: _ }) => {
            assert_eq!(provider, "gemini");
            assert_eq!(prompt, "Test prompt");
            assert_eq!(api_key, Some("test-key-123".to_string()));
        }
        _ => panic!("Expected Execute command"),
    }
}

#[test]
fn test_parse_pipeline_command() {
    let args = vec![
        "ai-cli",
        "--chain",
        "claude:設計 -> gemini:実装 -> codex:レビュー"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Pipeline { chain, context: _, no_stream }) => {
            assert_eq!(chain, "claude:設計 -> gemini:実装 -> codex:レビュー");
            assert!(!no_stream); // stream is true by default
        }
        _ => panic!("Expected Pipeline command"),
    }
}

#[test]
fn test_parse_execute_with_context_file() {
    let args = vec![
        "ai-cli",
        "--provider", "claude",
        "--prompt", "Analyze this",
        "--context", "file.txt"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Execute { provider: _, prompt: _, api_key: _, context, no_stream: _ }) => {
            assert_eq!(context, Some("file.txt".to_string()));
        }
        _ => panic!("Expected Execute command"),
    }
}

#[test]
fn test_parse_execute_no_stream() {
    let args = vec![
        "ai-cli",
        "--provider", "claude",
        "--prompt", "Hello",
        "--no-stream"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Execute { provider: _, prompt: _, api_key: _, context: _, no_stream }) => {
            assert!(no_stream);
        }
        _ => panic!("Expected Execute command"),
    }
}

#[test]
fn test_parse_pipeline_with_context() {
    let args = vec![
        "ai-cli",
        "--chain", "claude:analyze -> gemini:summarize",
        "--context", "data.json"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Pipeline { chain: _, context, no_stream: _ }) => {
            assert_eq!(context, Some("data.json".to_string()));
        }
        _ => panic!("Expected Pipeline command"),
    }
}

#[test]
fn test_parse_verbose_flag() {
    let args = vec![
        "ai-cli",
        "--provider", "claude",
        "--prompt", "test",
        "--verbose"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    assert!(cli_args.verbose);
}

#[test]
fn test_parse_quiet_flag() {
    let args = vec![
        "ai-cli",
        "--provider", "claude",
        "--prompt", "test",
        "--quiet"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    assert!(cli_args.quiet);
}

#[test]
fn test_parse_list_providers_command() {
    let args = vec!["ai-cli", "--list-providers"];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::ListProviders) => (),
        _ => panic!("Expected ListProviders command"),
    }
}

#[test]
fn test_parse_check_auth_command() {
    let args = vec!["ai-cli", "--check-auth", "claude"];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::CheckAuth { provider }) => {
            assert_eq!(provider, "claude");
        }
        _ => panic!("Expected CheckAuth command"),
    }
}

#[test]
fn test_parse_version_command() {
    let args = vec!["ai-cli", "--version"];
    let cli_args = CliArgs::parse_from(args);
    
    match cli_args.command {
        Some(Command::Version) => (),
        _ => panic!("Expected Version command"),
    }
}

#[test]
fn test_execute_command_helper() {
    let args = vec![
        "ai-cli",
        "--provider", "claude",
        "--prompt", "Hello",
        "--no-stream"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    if let Some(cmd) = &cli_args.command {
        if let Some(exec) = cmd.as_execute() {
            assert_eq!(exec.provider, "claude");
            assert_eq!(exec.prompt, "Hello");
            assert!(!exec.stream);
            assert!(exec.no_stream);
        } else {
            panic!("Expected Execute command");
        }
    }
}

#[test]
fn test_pipeline_command_helper() {
    let args = vec![
        "ai-cli",
        "--chain", "claude:test",
        "--context", "file.txt"
    ];
    let cli_args = CliArgs::parse_from(args);
    
    if let Some(cmd) = &cli_args.command {
        if let Some(pipe) = cmd.as_pipeline() {
            assert_eq!(pipe.chain, "claude:test");
            assert_eq!(pipe.context_file(), Some("file.txt".to_string()));
            assert!(pipe.stream);
        } else {
            panic!("Expected Pipeline command");
        }
    }
}