use ai_cli::pipeline::{PipelineStep, PipelineParser};

#[test]
fn test_parse_simple_pipeline() {
    let input = "claude:設計 -> gemini:実装 -> codex:レビュー";
    let steps = PipelineParser::parse(input).unwrap();
    
    assert_eq!(steps.len(), 3);
    
    assert_eq!(steps[0].provider, "claude");
    assert_eq!(steps[0].action, "設計");
    
    assert_eq!(steps[1].provider, "gemini");
    assert_eq!(steps[1].action, "実装");
    
    assert_eq!(steps[2].provider, "codex");
    assert_eq!(steps[2].action, "レビュー");
}

#[test]
fn test_parse_two_step_pipeline() {
    let input = "claude:analyze -> gemini:summarize";
    let steps = PipelineParser::parse(input).unwrap();
    
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].provider, "claude");
    assert_eq!(steps[0].action, "analyze");
    assert_eq!(steps[1].provider, "gemini");
    assert_eq!(steps[1].action, "summarize");
}

#[test]
fn test_parse_single_step() {
    let input = "claude:generate";
    let steps = PipelineParser::parse(input).unwrap();
    
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].provider, "claude");
    assert_eq!(steps[0].action, "generate");
}

#[test]
fn test_parse_with_spaces() {
    let input = "claude:設計->gemini:実装";
    let steps = PipelineParser::parse(input).unwrap();
    
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].provider, "claude");
    assert_eq!(steps[0].action, "設計");
}

#[test]
fn test_parse_with_extra_spaces() {
    let input = "  claude:design   ->   gemini:implement  ";
    let steps = PipelineParser::parse(input).unwrap();
    
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].provider, "claude");
    assert_eq!(steps[0].action, "design");
}

#[test]
fn test_parse_invalid_format_missing_colon() {
    let input = "claude -> gemini:test";
    let result = PipelineParser::parse(input);
    
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_format_empty_provider() {
    let input = ":action -> gemini:test";
    let result = PipelineParser::parse(input);
    
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_format_empty_action() {
    let input = "claude: -> gemini:test";
    let result = PipelineParser::parse(input);
    
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_string() {
    let input = "";
    let result = PipelineParser::parse(input);
    
    assert!(result.is_err());
}

#[test]
fn test_pipeline_step_creation() {
    let step = PipelineStep::new("claude", "analyze");
    
    assert_eq!(step.provider, "claude");
    assert_eq!(step.action, "analyze");
}

#[test]
fn test_pipeline_step_with_context() {
    let mut step = PipelineStep::new("claude", "design");
    step.set_context("Previous analysis results");
    
    assert_eq!(step.get_context(), Some("Previous analysis results".to_string()));
}

#[test]
fn test_pipeline_validate_providers() {
    let input = "claude:test -> unknown_provider:test";
    let steps = PipelineParser::parse(input).unwrap();
    
    let validation = PipelineParser::validate_providers(&steps, &["claude", "gemini", "codex"]);
    assert!(validation.is_err());
}

#[test]
fn test_pipeline_validate_all_known_providers() {
    let input = "claude:test -> gemini:test -> codex:test";
    let steps = PipelineParser::parse(input).unwrap();
    
    let validation = PipelineParser::validate_providers(&steps, &["claude", "gemini", "codex"]);
    assert!(validation.is_ok());
}