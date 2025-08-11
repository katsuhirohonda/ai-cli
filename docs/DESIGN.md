# AI CLI Aggregator Design Document

## Implementation Status

- [x] CLIインターフェース（clapパーサ）実装済み（`src/cli/mod.rs`）
- [x] コマンド実行との統合（`main.rs`でのディスパッチ）実装済み
- [x] Providerトレイト実装済み（`providers::AIProvider`）
- [x] プロバイダ: Claude 実装済み（`providers/claude.rs`）
- [x] プロバイダ: Gemini 実装済み（スタブ）
- [x] プロバイダ: Codex 実装済み（スタブ）
- [x] パイプラインDSLパーサ（`provider:action -> ...`）実装済み（`pipeline::PipelineParser`）
- [x] パイプライン実行エンジン（リトライ/エラー継続）実装済み（`pipeline::PipelineExecutor`）
- [x] ステップ間Transform 実装済み（`PipelineStep::transform`）
- [x] プロバイダ検証ユーティリティ（`validate_providers`）実装済み（未連携）
- [x] ストリーミング経路（簡易：`execute_streaming` は `execute` を委譲）実装済み
- [x] コンテキスト管理（履歴/ファイル/環境/メタ）実装済み（`providers::Context`）
- [x] 認証方式の型（ApiKey/Account/Cli/Browser）実装済み（`auth::AuthMethod`）
- [x] 認証検出：APIキー/CLIセッション検出 実装済み（一部：CLIは設定ファイル存在チェックのみ）
- [ ] 既存CLIの資格情報読み取り・設定ファイル統合 未実装
- [ ] 対話ログイン/設定ファイルフォールバック 未実装（設計の優先順序どおりの完全実装は未着手）
- [ ] フォールバックプロバイダ切替/サーキットブレーカ 未実装
- [x] 優雅な劣化（continue_on_error で継続・メタ付与）実装済み
- [x] CLIコマンド: Execute/Pipeline/list-providers/check-auth のパーサ実装済み（実行連携は未）
- [ ] CLIコマンド: Parallel/Interactive 未実装
- [ ] 設定ファイル（TOML）読み込み・パイプライン定義 未実装
- [ ] セキュリティ（セッション暗号化/自動更新）未実装

参考ファイル: `src/cli/mod.rs`, `src/pipeline/mod.rs`, `src/providers/mod.rs`, `src/providers/claude.rs`, `src/auth/mod.rs`, `src/main.rs`

## 1. Overview

### 1.1 Background
現在、開発者は複数のAI CLIツール（Claude Code、Gemini CLI、Codex CLI）を個別に使用している。各ツールには異なる強みがあるが、統一されたインターフェースがないため、効率的な活用が困難である。

### 1.2 Goals
- 複数のAI CLIツールを統一インターフェースで提供
- パイプライン処理による各AIの強みを活かした連携
- 開発ワークフローの自動化と効率化
- 既存CLIツールの認証情報を活用した簡単なセットアップ

### 1.3 Non-Goals
- 各AI CLIツールの再実装
- GUIインターフェースの提供
- プロプライエタリなAIモデルの開発

## 2. System Architecture

### 2.1 High-Level Design
```
┌─────────────────────────────────────────────────┐
│                  CLI Interface                   │
│              (clap-based parser)                 │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│             Command Dispatcher                   │
│         (Router & Pipeline Engine)               │
└─────────────────┬───────────────────────────────┘
                  │
        ┌─────────┴─────────┬─────────────┐
        ▼                   ▼             ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│Provider Adapter│  │Provider Adapter│  │Provider Adapter│
│  (Claude)      │  │  (Gemini)      │  │  (Codex)      │
└───────────────┘  └───────────────┘  └───────────────┘
```

### 2.2 Core Components

#### Provider Trait
```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn execute(&self, prompt: &str, context: &Context) -> Result<Response>;
    async fn stream(&self, prompt: &str, context: &Context) -> Result<ResponseStream>;
    fn capabilities(&self) -> Capabilities;
    fn name(&self) -> &str;
}
```

#### Pipeline Executor
```rust
pub struct Pipeline {
    steps: Vec<PipelineStep>,
    error_strategy: ErrorStrategy,
}

pub struct PipelineStep {
    provider: Box<dyn AIProvider>,
    action: String,
    transform: Option<Box<dyn Transform>>,
}
```

## 3. Detailed Design

### 3.1 Pipeline Processing

#### Pipeline DSL Syntax
```
chain := step ( "->" step )*
step  := provider ":" action
```

#### Execution Flow
1. Parse pipeline definition
2. Validate provider availability
3. Execute steps sequentially
4. Apply transformations between steps
5. Handle errors according to strategy
6. Return aggregated result

### 3.2 Context Management
```rust
pub struct Context {
    conversation_history: Vec<Message>,
    current_files: Vec<PathBuf>,
    environment: HashMap<String, String>,
    metadata: HashMap<String, Value>,
}
```

### 3.3 Authentication Management

#### Multiple Authentication Methods
```rust
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
    CliAuth {
        // 各CLIツールの既存認証を利用
    },
}
```

#### Authentication Flow
1. **既存CLIの認証を優先利用**
   - `claude login` の既存セッションを検出・利用
   - `gemini auth` の認証情報を参照
   - 各ツールのconfig/credentialsファイルを読み込み

2. **フォールバック順序**
   ```
   既存CLIセッション → 環境変数 → 設定ファイル → インタラクティブログイン
   ```

### 3.4 Error Handling
- **Retry**: 指数バックオフによるリトライ
- **Fallback**: 代替プロバイダーへの切り替え
- **Circuit Breaker**: 連続失敗時の自動遮断
- **Graceful Degradation**: 部分的な結果の返却

## 4. API Design

### 4.1 CLI Commands
```bash
# Single provider
ai chat --provider claude "prompt"
ai code --provider codex "generate function"

# Pipeline execution
ai --chain "claude:design -> codex:implement -> gemini:review"
ai --pipeline development "requirements"

# Parallel execution
ai --parallel claude,gemini "prompt"

# Interactive mode
ai chat --provider claude --interactive
```

### 4.2 Configuration Format
```toml
[providers.claude]
# API keyは任意 - 既存のClaude CLIセッションを自動検出
api_key = "${CLAUDE_API_KEY}"  # Optional
use_cli_auth = true  # デフォルト: true

[providers.gemini]
# Gemini CLIの認証を利用
use_cli_auth = true
api_key = "${GEMINI_API_KEY}"  # Optional

[providers.codex]
# 複数の認証方式から選択
auth_method = "cli"  # "cli" | "api_key" | "browser"

[defaults]
provider = "claude"
timeout = 30
retry_count = 3

[pipelines.development]
steps = [
    { provider = "claude", action = "design" },
    { provider = "codex", action = "implement" },
    { provider = "gemini", action = "review" }
]
```

## 5. Data Flow

### 5.1 Single Provider Flow
```
User Input → Parser → Provider Selection → Execution → Output
```

### 5.2 Pipeline Flow
```
User Input → Pipeline Parser → Step 1 → Transform → Step 2 → ... → Aggregate → Output
```

## 6. Security Considerations

- **セッション管理**
  - 既存CLIツールのセッション情報を安全に読み取り
  - セッショントークンのメモリ内暗号化
  - 自動セッション更新

- **認証情報の保護**
  - API keyは環境変数で管理（任意）
  - セッション情報はOSのキーチェーン/資格情報マネージャーを利用
  - 認証情報のログ出力禁止

- **権限の分離**
  - 各プロバイダーの認証は独立管理
  - 最小権限の原則を適用

## 7. Performance Requirements

- Response time: < 100ms overhead per request
- Pipeline execution: Parallel where possible
- Memory usage: < 50MB baseline
- Caching: LRU with 1GB limit

## 8. Testing Strategy

### 8.1 Test-Driven Development (TDD)
- **Red-Green-Refactor サイクル**
  - 失敗するテストを最初に書く
  - テストを通す最小限の実装
  - リファクタリング

### 8.2 Unit Tests
- Provider adapters
- Pipeline parser
- Transform functions
- Authentication managers

### 8.3 Integration Tests
- End-to-end pipeline execution
- Error handling scenarios
- Provider failover
- Authentication flow

### 8.4 Performance Tests
- Latency benchmarks
- Memory profiling
- Concurrent request handling

## 9. Implementation Phases

### Phase 1: Foundation
- [x] Rustプロジェクト初期化
- [x] Provider trait定義（`providers::AIProvider`）
- [x] 基本的なCLIパーサー（`src/cli/mod.rs`）

### Phase 2: Provider Integration
- [x] Claude Code adapter（`providers/claude.rs`）
- [x] Gemini CLI adapter（スタブ実装）
- [x] Codex CLI adapter（スタブ実装）
- [x] Authentication manager（`auth/mod.rs`）

### Phase 3: Pipeline Engine
- [x] Pipeline DSL parser（`pipeline::PipelineParser`）
- [x] Pipeline execution engine（`pipeline::PipelineExecutor`）
- [x] Context management（`providers::Context`）
- [x] Transform functions（`PipelineStep::transform`）

### Phase 4: Advanced Features
- [ ] Parallel execution
- [ ] Caching mechanism
- [ ] Configuration system
- [ ] Error recovery

### Phase 5: Polish & Release
- [ ] Comprehensive testing
- [ ] Documentation
- [ ] Performance optimization
- [ ] CI/CD setup

## 10. Dependencies

```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"
reqwest = { version = "0.12", features = ["stream"] }
async-trait = "0.1"
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
indicatif = "0.17"
colored = "2.1"
nom = "7.1"
dashmap = "6.0"
futures = "0.3"
dirs = "5.0"

[dev-dependencies]
mockall = "0.13"
tokio-test = "0.4"
pretty_assertions = "1.4"
```

## 11. Example Usage

### Basic Commands
```bash
# 単一プロバイダー実行
ai chat --provider claude "Rustでファイル操作のコードを書いて"

# パイプライン実行
ai --chain "claude:要件定義 -> codex:実装 -> gemini:レビュー" "ECサイトの商品管理機能"

# 並列実行
ai --parallel claude,gemini "最適なソートアルゴリズムは？"

# インタラクティブモード
ai chat --provider claude --interactive
```

### Pipeline Definition Example
```yaml
# pipelines/full-development.yaml
name: full_development
description: "Complete development cycle"
steps:
  - provider: claude
    action: design
    prompt: |
      以下の要件に基づいて設計してください：
      {input}
  
  - provider: codex
    action: implement
    prompt: |
      上記の設計に基づいて実装してください
    
  - provider: gemini
    action: review
    prompt: |
      実装をレビューして改善点を指摘してください
```

## 12. Future Enhancements

- プラグインシステム
- カスタムプロバイダーのサポート
- Web API提供
- VSCode拡張機能
- GitHub Actions統合
