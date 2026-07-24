# Phase 1 实现文档

本文档只描述 Phase 1 MVP（Web 应用）的具体实现：目录结构、每个文件的完整代码、少量配置与说明文字。Phase 2/3 的内容不在本文档范围内。

---

## 0. 里程碑与验收

| 里程碑 | 范围 | 验收标准 |
|---|---|---|
| M1 后端骨架 | Cargo 工作区、axum server、DeepSeek 客户端、health 路由 | `curl /api/health` 返回 `ok`；`DEEPSEEK_API_KEY` 设置后能成功调用一次 chat completions |
| M2 Lean4 集成 | `lean` 子进程调用、临时文件、stderr 解析 | 给定 `theorem t : 1+1=2 := by rfl` 返回 `success:true`；给定错误代码返回带行号的错误 |
| M3 Agent Loop | 工具调用循环、System Prompt、5 个工具、thinking 模式处理 | 用户输入一道数学题，Agent 自主调用 `run_lean_check` 直至编译通过，最后给出自然语言解释 |
| M4 前端界面 | React + Vite、对话面板、CodeMirror 编辑器、证明状态 | 浏览器中完成一次完整对话，能看到 thinking、tool_call、tool_result、answer 四类事件 |
| M5 开源发布 | README、`.env.example`、LICENSE、`.gitignore` | 按 README 步骤在干净机器上从零启动成功 |

---

## 1. 仓库目录结构

仓库根 `deepseek-leanspark/`（即当前文件夹）只承载导航与跨阶段共享资源；Phase 1 的全部代码、配置、Prompt、前端、测试都放在 `DeepSeek-LeanSpark/` 子目录下；阶段文档放在 `docs/`。这样 Phase 2 可以直接在 `DeepSeek-LeanSpark/` 内部加 `src-tauri/` 等子目录演进，无需复制 Phase 1 代码；测试代码独立到 `DeepSeek-LeanSpark/tests/`（Rust 集成测试标准位置，`cargo test` 自动识别）。

```
deepseek-leanspark/                      # 仓库根（当前文件夹）
├── README.md                            # 仓库总览，导航到各阶段文档
├── docs/                                # 跨阶段文档，独立于代码
│   ├── phase1.md                        # 本文档
│   ├── phase2.md                        # （未来）
│   └── leanspark-guide.html             # 项目知识基线
├── DeepSeek-LeanSpark/                  # 项目主代码
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── .env.example
│   ├── .gitignore
│   ├── LICENSE
│   ├── README.md                        # 项目级 README（启动步骤等）
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   └── routes.rs
│   │   ├── agent/
│   │   │   ├── mod.rs
│   │   │   ├── agent_loop.rs
│   │   │   └── prompt.rs
│   │   ├── deepseek/
│   │   │   ├── mod.rs
│   │   │   └── client.rs
│   │   ├── lean/
│   │   │   ├── mod.rs
│   │   │   └── runner.rs
│   │   └── tools/
│   │       ├── mod.rs
│   │       ├── lean_check.rs
│   │       ├── search.rs
│   │       └── proof_state.rs
│   ├── prompts/
│   │   └── agent-prompt.md
│   ├── frontend/
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── vite.config.ts
│   │   ├── index.html
│   │   └── src/
│   │       ├── main.tsx
│   │       ├── App.tsx
│   │       ├── types.ts
│   │       ├── hooks/
│   │       │   └── useAgent.ts
│   │       └── components/
│   │           ├── ChatPanel.tsx
│   │           ├── CodeEditor.tsx
│   │           └── ProofState.tsx
│   └── tests/                           # Rust 集成测试（cargo test 自动识别）
│       └── api_smoke.rs                 # M1-M2 烟雾测试示例
└── scripts/                             # CI/发布/安装脚本（未来）
```

> **关于阶段隔离**：Phase 1 与 Phase 2 不做物理目录隔离。Phase 2 在 `DeepSeek-LeanSpark/` 内部加 `src-tauri/` 子目录、修改 `Cargo.toml` workspace 配置即可演进。阶段里程碑用 git tag 标记（如 `phase1-m5-release`），不靠平行目录。

代码模块依赖关系（位于 `DeepSeek-LeanSpark/src/` 内）：

```
deepseek/  lean/   tools/
     ↘      ↓      ↙
       agent_loop
            ↓
            api/routes
            ↓
          lib.rs (AppState) ← main.rs
            ↑
        frontend (HTTP)
```

---

## 2. 后端代码

### 2.1 `Cargo.toml`

```toml
[package]
name = "deepseek-leanspark"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
description = "AI-assisted formal mathematics: DeepSeek + Lean4"
license = "MIT"

[lib]
name = "deepseek_leanspark"
path = "src/lib.rs"

[[bin]]
name = "deepseek-leanspark"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.8", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "fs", "trace"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4"] }
dotenvy = "0.15"
thiserror = "1"
anyhow = "1"
async-trait = "0.1"
once_cell = "1"

[dev-dependencies]
# 端到端烟雾测试用 blocking 客户端，见 tests/api_smoke.rs
reqwest = { version = "0.12", features = ["json", "rustls-tls", "blocking"], default-features = false }

[profile.release]
opt-level = 3
lto = true
strip = true
codegen-units = 1
```

要点：
- `reqwest` 关闭 default features、改用 `rustls-tls`，避免 Windows 上 OpenSSL 链接问题。
- `axum = "0.8"` 的 `axum::serve` 需要 `tokio::net::TcpListener`。
- `async-trait` 用于 `Tool` trait 的 async 方法。
- `lib + bin` 双产物：bin 给 Phase 1 直接跑，lib 留给 Phase 2 Tauri 复用。

### 2.2 `src/main.rs`

```rust
use deepseek_leanspark::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
```

### 2.3 `src/lib.rs`

```rust
pub mod agent;
pub mod api;
pub mod deepseek;
pub mod lean;
pub mod tools;

pub use agent::AgentLoop;
pub use deepseek::DeepSeekClient;
pub use lean::LeanRunner;

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub deepseek: Arc<DeepSeekClient>,
    pub lean: Arc<LeanRunner>,
    pub tools: Arc<tools::ToolRegistry>,
    pub agent: Arc<AgentLoop>,
}

pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY must be set (see .env.example)");
    let model = std::env::var("DEEPSEEK_MODEL")
        .unwrap_or_else(|_| "deepseek-v4-flash".to_string());
    let lean_path = std::env::var("LEAN_BIN_PATH")
        .unwrap_or_else(|_| "lean".to_string());
    let addr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let deepseek = Arc::new(DeepSeekClient::new(api_key, model));
    let lean = Arc::new(LeanRunner::new(lean_path));
    let tools = Arc::new(tools::ToolRegistry::new(lean.clone()));
    let agent = Arc::new(AgentLoop::new(deepseek.clone(), tools.clone()));

    let state = AppState { deepseek, lean, tools, agent };

    let app = api::routes::router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### 2.4 `src/deepseek/mod.rs`

```rust
mod client;

pub use client::{ChatResponse, DeepSeekClient, ToolCall};
```

### 2.5 `src/deepseek/client.rs`

```rust
use anyhow::Result;
use serde_json::Value;
use std::time::Duration;

const BASE_URL: &str = "https://api.deepseek.com";

#[derive(Clone)]
pub struct DeepSeekClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .expect("failed to build reqwest client");
        Self { client, api_key, model }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// 普通模式聊天补全
    pub async fn chat(
        &self,
        messages: &[Value],
        tools: Option<&[Value]>,
        tool_choice: Option<&str>,
    ) -> Result<ChatResponse> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });
        if let Some(t) = tools {
            body["tools"] = Value::Array(t.to_vec());
        }
        if let Some(tc) = tool_choice {
            body["tool_choice"] = Value::String(tc.to_string());
        }
        self.call(body).await
    }

    /// 思考模式聊天补全
    /// thinking 模式下回传 assistant 消息必须含 reasoning_content
    pub async fn chat_with_thinking(
        &self,
        messages: &[Value],
        tools: &[Value],
        reasoning_effort: &str,
    ) -> Result<ChatResponse> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "reasoning_effort": reasoning_effort,
            "thinking": { "type": "enabled" }
        });
        self.call(body).await
    }

    async fn call(&self, body: Value) -> Result<ChatResponse> {
        let resp = self
            .client
            .post(format!("{}/chat/completions", BASE_URL))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("DeepSeek API error {}: {}", status, text);
        }
        let value: Value = serde_json::from_str(&text)?;
        ChatResponse::from_value(value)
    }
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub finish_reason: String,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // 原始 JSON 字符串
}

impl ChatResponse {
    pub fn from_value(v: Value) -> Result<Self> {
        let choice = v
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| anyhow::anyhow!("missing choices[0] in response: {}", v))?;
        let message = choice
            .get("message")
            .ok_or_else(|| anyhow::anyhow!("missing message"))?;
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("stop")
            .to_string();
        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let reasoning_content = message
            .get("reasoning_content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tool_calls = message
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let name = tc.get("function")?.get("name")?.as_str()?.to_string();
                        let args = tc.get("function")?.get("arguments")?.as_str()?.to_string();
                        Some(ToolCall { id, name, arguments: args })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self { finish_reason, content, reasoning_content, tool_calls })
    }

    /// 转换为可推回 messages 的 assistant 消息
    /// 关键：thinking 模式下回传必须含 reasoning_content，否则 400
    pub fn to_assistant_message(&self) -> Value {
        let mut msg = serde_json::json!({
            "role": "assistant",
            "content": self.content.clone(),
        });
        if let Some(rc) = &self.reasoning_content {
            msg["reasoning_content"] = Value::String(rc.clone());
        }
        if !self.tool_calls.is_empty() {
            msg["tool_calls"] = Value::Array(
                self.tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments
                            }
                        })
                    })
                    .collect(),
            );
        }
        msg
    }
}
```

### 2.6 `src/lean/mod.rs`

```rust
mod runner;

pub use runner::{LeanResult, LeanRunner};
```

### 2.7 `src/lean/runner.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone)]
pub struct LeanRunner {
    lean_bin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub contains_sorry: bool,
}

impl LeanRunner {
    pub fn new(lean_bin: String) -> Self {
        Self { lean_bin }
    }

    /// 写入临时文件，执行 `lean <file>`，返回编译结果
    pub async fn run(&self, code: &str) -> Result<LeanResult> {
        let tmp = std::env::temp_dir().join(format!("leanspark_{}.lean", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp, code).await?;

        let output = Command::new(&self.lean_bin).arg(&tmp).output().await;

        // 清理临时文件（即使执行失败）
        let _ = tokio::fs::remove_file(&tmp).await;

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                return Ok(LeanResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "failed to execute lean: {}. Is '{}' on PATH? Set LEAN_BIN_PATH in .env.",
                        e, self.lean_bin
                    )),
                    contains_sorry: false,
                })
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Lean 把诊断信息写到 stderr
        let combined = if stderr.is_empty() { stdout.clone() } else { stderr.clone() };
        let contains_sorry = code.contains("sorry") || code.contains("admit");
        let success = output.status.success() && stderr.is_empty();

        Ok(LeanResult {
            success,
            output: if success { "no errors".to_string() } else { combined },
            error: if success { None } else { Some(combined) },
            contains_sorry,
        })
    }

    pub async fn check_file(&self, path: &PathBuf) -> Result<LeanResult> {
        let code = tokio::fs::read_to_string(path).await?;
        self.run(&code).await
    }
}
```

### 2.8 `src/tools/mod.rs`

```rust
use crate::lean::LeanRunner;
use crate::tools::lean_check::LeanCheckTool;
use crate::tools::proof_state::ProofStateTool;
use crate::tools::search::SearchMathlibTool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub mod lean_check;
pub mod proof_state;
pub mod search;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn spec(&self) -> Value;
    async fn call(&self, args: &Value) -> Result<String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new(lean: Arc<LeanRunner>) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        let lean_check = Arc::new(LeanCheckTool::new(lean));
        let search = Arc::new(SearchMathlibTool::new());
        let proof_state = Arc::new(ProofStateTool::new());
        tools.insert(lean_check.name().to_string(), lean_check);
        tools.insert(search.name().to_string(), search);
        tools.insert(proof_state.name().to_string(), proof_state);
        Self { tools }
    }

    pub fn specs(&self) -> Vec<Value> {
        self.tools.values().map(|t| t.spec()).collect()
    }

    pub async fn dispatch(&self, name: &str, args: &Value) -> Result<String> {
        match self.tools.get(name) {
            Some(tool) => tool.call(args).await,
            None => anyhow::bail!("unknown tool: {}", name),
        }
    }
}
```

### 2.9 `src/tools/lean_check.rs`

```rust
use crate::lean::LeanRunner;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct LeanCheckTool {
    lean: Arc<LeanRunner>,
}

impl LeanCheckTool {
    pub fn new(lean: Arc<LeanRunner>) -> Self {
        Self { lean }
    }
}

#[async_trait]
impl Tool for LeanCheckTool {
    fn name(&self) -> &str {
        "run_lean_check"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "run_lean_check",
                "description": "提交 Lean4 代码给编译器验证。返回 {success, output, warning}。success=true 表示编译通过。",
                "strict": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lean_code": {
                            "type": "string",
                            "description": "完整的 Lean4 代码，包含所有 import 与待验证的 theorem/lemma。禁止使用 sorry 或 admit。"
                        }
                    },
                    "required": ["lean_code"],
                    "additionalProperties": false
                }
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let lean_code = args
            .get("lean_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing lean_code argument"))?;

        let result = self.lean.run(lean_code).await?;
        let warning = if result.contains_sorry {
            " [WARNING: 代码包含 sorry 或 admit，这违反安全规则——禁止使用 sorry 跳过证明。请用真实 tactic 完成证明。]"
        } else {
            ""
        };
        let resp = json!({
            "success": result.success,
            "output": result.output,
            "warning": warning
        });
        Ok(serde_json::to_string(&resp)?)
    }
}
```

### 2.10 `src/tools/search.rs`

```rust
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct SearchMathlibTool {
    common_lemmas: Vec<(&'static str, &'static str)>,
}

impl SearchMathlibTool {
    pub fn new() -> Self {
        Self {
            // Phase 1：内置常用引理索引。Phase 3 将接入完整 mathlib 搜索。
            common_lemmas: vec![
                ("add_comm", "a + b = b + a"),
                ("add_assoc", "(a + b) + c = a + (b + c)"),
                ("add_zero", "a + 0 = a"),
                ("zero_add", "0 + a = a"),
                ("mul_comm", "a * b = b * a"),
                ("mul_assoc", "(a * b) * c = a * (b * c)"),
                ("mul_one", "a * 1 = a"),
                ("one_mul", "1 * a = a"),
                ("add_left_cancel", "a + b = a + c → b = c"),
                ("add_right_cancel", "b + a = c + a → b = c"),
                ("le_refl", "a ≤ a"),
                ("le_trans", "a ≤ b → b ≤ c → a ≤ c"),
                ("lt_of_le_of_lt", "a ≤ b → b < c → a < c"),
                ("eq_refl", "a = a"),
                ("Ne.symm", "a ≠ b → b ≠ a"),
                ("Iff.refl", "P ↔ P"),
                ("True.intro", "True"),
                ("False.elim", "False → P"),
                ("Classical.by_contradiction", "¬¬P → P"),
                ("by_contra", "反证法 tactic"),
                ("Nat.succ_eq_add_one", "n.succ = n + 1"),
                ("Nat.add_zero", "n + 0 = n"),
                ("Nat.zero_add", "0 + n = n"),
                ("Nat.mul_one", "n * 1 = n"),
                ("Nat.one_mul", "1 * n = n"),
                ("Nat.le_refl", "n ≤ n"),
                ("Continuous.comp", "连续函数复合连续"),
                ("Continuous.add", "两连续函数之和连续"),
                ("Continuous.mul", "两连续函数之积连续"),
                ("Continuous.neg", "连续函数取负连续"),
                ("Continuous.sub", "两连续函数之差连续"),
                ("Real.continuous_pow", "实数幂函数连续"),
                ("Real.continuous_abs", "绝对值连续"),
                ("Monotone.add", "两单调函数之和单调"),
                ("Monotone.mul_of_nonneg", "非负系数下单调"),
                ("Filter.tendsto_add", "极限的和等于和的极限"),
                ("Filter.tendsto_mul", "极限的积等于积的极限"),
                ("tendsto_const", "常数列收敛到自身"),
                ("Finset.sum_add_distrib", "求和分配到加法"),
                ("Finset.sum_mul", "求和与乘法"),
                ("Finset.card_union", "并集基数"),
                ("Set.union_comm", "集合并交换律"),
                ("Set.inter_comm", "集合交交换律"),
                ("Set.subset_def", "子集定义"),
                ("List.map", "列表映射"),
                // 常用 tactic
                ("ring", "环等式 tactic"),
                ("norm_num", "数值规范化 tactic"),
                ("decide", "决策过程 tactic"),
                ("simp", "化简器 tactic"),
                ("rw", "重写 tactic"),
                ("rewrite", "重写 tactic"),
                ("exact", "精确匹配 tactic"),
                ("apply", "应用 tactic"),
                ("induction", "归纳 tactic"),
                ("cases", "分情况 tactic"),
                ("use", "提供见证 tactic"),
                ("have", "引入中间结论 tactic"),
                ("let", "引入绑定 tactic"),
                ("calc", "计算证明 tactic"),
                ("constructor", "构造器 tactic"),
                ("rcases", "递归分情况 tactic"),
                ("obtain", "获取 tactic"),
                ("refine", "细化 tactic"),
                ("simp only", "只化简指定引理 tactic"),
            ],
        }
    }
}

#[async_trait]
impl Tool for SearchMathlibTool {
    fn name(&self) -> &str {
        "search_mathlib"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "search_mathlib",
                "description": "搜索 mathlib 中的定理/引理/tactic 名称及其简述。Phase 1 提供常见引理索引。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索关键词，例如 'continuous'、'add_comm'、'monotone'"
                        }
                    },
                    "required": ["query"]
                }
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let matches: Vec<_> = self
            .common_lemmas
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&query))
            .collect();

        if matches.is_empty() {
            Ok(serde_json::to_string(&json!({
                "success": false,
                "message": format!(
                    "未找到匹配 '{}' 的引理。建议尝试关键词: continuous, add, mul, monotone, comm, assoc, le, lt, tendsto, simp, rw, ring"
                ),
            }))?)
        } else {
            let results: Vec<Value> = matches
                .iter()
                .map(|(name, desc)| json!({ "name": name, "description": desc }))
                .collect();
            Ok(serde_json::to_string(&json!({
                "success": true,
                "results": results,
                "note": "Phase 1 仅提供常见引理索引。Phase 3 将接入完整 mathlib 全文搜索。"
            }))?)
        }
    }
}
```

### 2.11 `src/tools/proof_state.rs`

```rust
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ProofStateTool;

impl ProofStateTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ProofStateTool {
    fn name(&self) -> &str {
        "get_proof_state"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "get_proof_state",
                "description": "从 Lean4 代码中提取证明状态信息。Phase 1 通过静态解析提取 theorem/lemma 声明、是否含 sorry、by 块数量。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lean_code": {
                            "type": "string",
                            "description": "Lean4 代码"
                        }
                    },
                    "required": ["lean_code"]
                }
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let code = args
            .get("lean_code")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut declarations = Vec::new();
        for line in code.lines() {
            let trimmed = line.trim();
            let kind_and_rest = trimmed
                .strip_prefix("theorem ")
                .map(|r| ("theorem", r))
                .or_else(|| trimmed.strip_prefix("lemma ").map(|r| ("lemma", r)));
            if let Some((kind, rest)) = kind_and_rest {
                let name = rest.split_whitespace().next().unwrap_or("");
                if !name.is_empty() {
                    declarations.push(json!({ "kind": kind, "name": name }));
                }
            }
        }

        let has_sorry = code.contains("sorry") || code.contains("admit");
        let by_count = code.matches(" by ").count();

        Ok(serde_json::to_string(&json!({
            "success": true,
            "declarations": declarations,
            "has_sorry": has_sorry,
            "by_blocks_count": by_count,
            "note": "Phase 1 为静态解析。Phase 2 将通过 LSP 提供实时 goal state。"
        }))?)
    }
}
```

### 2.12 `src/agent/mod.rs`

```rust
pub mod agent_loop;
pub mod prompt;

pub use agent_loop::{AgentEvent, AgentLoop};
pub use prompt::load_system_prompt;
```

### 2.13 `src/agent/prompt.rs`

```rust
use anyhow::Result;

pub fn load_system_prompt() -> Result<String> {
    // 编译期把 prompts/agent-prompt.md 内联进二进制
    let prompt = include_str!("../../prompts/agent-prompt.md");
    Ok(prompt.to_string())
}
```

### 2.14 `src/agent/agent_loop.rs`

```rust
use crate::deepseek::{ChatResponse, DeepSeekClient};
use crate::tools::ToolRegistry;
use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;

const MAX_ITERATIONS: usize = 20;

pub struct AgentLoop {
    client: Arc<DeepSeekClient>,
    tools: Arc<ToolRegistry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentEvent {
    pub kind: String, // "thinking" | "tool_call" | "tool_result" | "answer" | "error"
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<Value>,
}

impl AgentLoop {
    pub fn new(client: Arc<DeepSeekClient>, tools: Arc<ToolRegistry>) -> Self {
        Self { client, tools }
    }

    /// 运行 Agent 循环
    /// - user_message: 用户本轮输入
    /// - history: 之前的完整消息历史（不含 system prompt）
    /// - use_thinking: 是否启用 thinking 模式
    /// 返回 (本轮产生的事件列表, 完整的新消息历史含 system prompt)
    pub async fn run(
        &self,
        user_message: &str,
        history: &[Value],
        use_thinking: bool,
    ) -> Result<(Vec<AgentEvent>, Vec<Value>)> {
        let system_prompt = crate::agent::prompt::load_system_prompt()?;
        let tool_specs = self.tools.specs();

        let mut messages: Vec<Value> = Vec::with_capacity(history.len() + 2);
        messages.push(json!({ "role": "system", "content": system_prompt }));
        for h in history {
            messages.push(h.clone());
        }
        messages.push(json!({ "role": "user", "content": user_message }));

        let mut events = Vec::new();

        for i in 0..MAX_ITERATIONS {
            tracing::info!("agent iteration {}/{}", i + 1, MAX_ITERATIONS);
            let resp: ChatResponse = if use_thinking {
                self.client
                    .chat_with_thinking(&messages, &tool_specs, "high")
                    .await?
            } else {
                self.client
                    .chat(&messages, Some(&tool_specs), Some("auto"))
                    .await?
            };

            if let Some(rc) = &resp.reasoning_content {
                events.push(AgentEvent {
                    kind: "thinking".to_string(),
                    content: rc.clone(),
                    tool_name: None,
                    tool_args: None,
                });
            }

            match resp.finish_reason.as_str() {
                "stop" => {
                    let answer = resp.content.clone().unwrap_or_default();
                    events.push(AgentEvent {
                        kind: "answer".to_string(),
                        content: answer,
                        tool_name: None,
                        tool_args: None,
                    });
                    messages.push(resp.to_assistant_message());
                    return Ok((events, messages));
                }
                "tool_calls" => {
                    // thinking 模式下必须把 reasoning_content 一起回传，否则 400
                    messages.push(resp.to_assistant_message());

                    for tc in &resp.tool_calls {
                        events.push(AgentEvent {
                            kind: "tool_call".to_string(),
                            content: format!("调用工具: {}", tc.name),
                            tool_name: Some(tc.name.clone()),
                            tool_args: Some(
                                serde_json::from_str(&tc.arguments).unwrap_or(Value::Null),
                            ),
                        });

                        let args: Value =
                            serde_json::from_str(&tc.arguments).unwrap_or(Value::Null);
                        let result = match self.tools.dispatch(&tc.name, &args).await {
                            Ok(output) => output,
                            Err(e) => {
                                let err_msg = format!("工具执行错误: {}", e);
                                events.push(AgentEvent {
                                    kind: "error".to_string(),
                                    content: err_msg.clone(),
                                    tool_name: Some(tc.name.clone()),
                                    tool_args: None,
                                });
                                json!({ "error": err_msg }).to_string()
                            }
                        };

                        events.push(AgentEvent {
                            kind: "tool_result".to_string(),
                            content: result.clone(),
                            tool_name: Some(tc.name.clone()),
                            tool_args: None,
                        });

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tc.id,
                            "content": result,
                        }));
                    }
                }
                other => {
                    events.push(AgentEvent {
                        kind: "error".to_string(),
                        content: format!("意外的 finish_reason: {}", other),
                        tool_name: None,
                        tool_args: None,
                    });
                    return Ok((events, messages));
                }
            }
        }

        events.push(AgentEvent {
            kind: "error".to_string(),
            content: format!("达到最大迭代次数 {}，Agent 循环终止", MAX_ITERATIONS),
            tool_name: None,
            tool_args: None,
        });
        Ok((events, messages))
    }
}
```

### 2.15 `src/api/mod.rs`

```rust
pub mod routes;
```

### 2.16 `src/api/routes.rs`

> 路径说明：本节起所有 `src/...` 路径均相对于 `DeepSeek-LeanSpark/`，即完整路径为 `DeepSeek-LeanSpark/src/api/routes.rs`，以此类推。

```rust
use crate::AppState;
use axum::{
    extract::{self, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/lean/check", post(lean_check))
        .route("/api/tools", get(list_tools))
        .route("/api/models", get(list_models))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    history: Vec<Value>,
    #[serde(default)]
    thinking: bool,
}

#[derive(Serialize)]
struct ChatResponseDto {
    events: Vec<crate::agent::AgentEvent>,
    messages: Vec<Value>,
}

async fn chat(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<ChatRequest>,
) -> Result<Json<ChatResponseDto>, (axum::http::StatusCode, String)> {
    let (events, messages) = state
        .agent
        .run(&req.message, &req.history, req.thinking)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ChatResponseDto { events, messages }))
}

#[derive(Deserialize)]
struct LeanCheckRequest {
    code: String,
}

async fn lean_check(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<LeanCheckRequest>,
) -> Result<Json<crate::lean::LeanResult>, (axum::http::StatusCode, String)> {
    let result = state
        .lean
        .run(&req.code)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(result))
}

async fn list_tools(State(state): State<AppState>) -> Json<Vec<Value>> {
    Json(state.tools.specs())
}

async fn list_models(State(state): State<AppState>) -> Json<Value> {
    Json(serde_json::json!({
        "current": state.deepseek.model(),
        "candidates": ["deepseek-v4-pro", "deepseek-v4-flash"]
    }))
}
```

### 2.17 `tests/api_smoke.rs`

Rust 集成测试位于 `DeepSeek-LeanSpark/tests/`，`cargo test` 会自动识别。下面给出 M1-M2 的烟雾测试示例，覆盖 health、tools 列表、Lean 编译成功/失败/sorry 检测三条路径。M3 涉及 DeepSeek API 真实调用，不放入默认测试套件，由 7.4 的 curl 脚本验收。

```rust
// tests/api_smoke.rs
// 运行：cargo test --test api_smoke
// 前置：先启动后端（cargo run），监听 127.0.0.1:3000
//      且 LEAN_BIN_PATH 指向可执行的 lean

use serde_json::Value;

const BASE: &str = "http://127.0.0.1:3000";

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

#[test]
fn health_returns_ok() {
    let body = client().get(format!("{}/api/health", BASE)).send().unwrap().text().unwrap();
    assert_eq!(body, "ok");
}

#[test]
fn tools_list_includes_run_lean_check() {
    let resp = client()
        .get(format!("{}/api/tools", BASE))
        .send()
        .unwrap();
    assert!(resp.status().is_success());
    let arr: Vec<Value> = resp.json().unwrap();
    let names: Vec<String> = arr
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"run_lean_check".to_string()));
    assert!(names.contains(&"search_mathlib".to_string()));
    assert!(names.contains(&"get_proof_state".to_string()));
}

#[test]
fn lean_check_passes_valid_rfl() {
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : 1 + 1 = 2 := by rfl"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["contains_sorry"], false);
}

#[test]
fn lean_check_fails_on_wrong_goal() {
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : 1 + 1 = 3 := by rfl"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);
    assert!(v["error"].as_str().unwrap().contains("type mismatch"));
}

#[test]
fn lean_check_flags_sorry() {
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : False := by sorry"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    // 代码含 sorry → contains_sorry=true，warning 通过 /api/chat 才可见
    assert_eq!(v["contains_sorry"], true);
}
```

> 测试依赖 `reqwest` 的 `blocking` feature，已在 2.1 节 `Cargo.toml` 的 `[dev-dependencies]` 段声明。

> 运行方式：先 `cargo run` 启动后端，新开终端 `cargo test --test api_smoke`。这些测试不是单元测试而是端到端烟雾测试，依赖后端在 3000 端口运行。

---

## 3. System Prompt

### 3.1 `prompts/agent-prompt.md`

```markdown
你是 DeepSeek-LeanSpark 的形式化证明助手。你的任务是帮助数学工作者把数学命题用 Lean4 形式化，并通过 Lean4 编译器的验证。

# 工作流程

1. 仔细阅读用户的数学问题。
2. 如不确定某个 Lean4 引理是否存在，先调用 `search_mathlib` 查找。
3. 起草完整的 Lean4 代码（包含所有 import），调用 `run_lean_check` 提交编译。
4. 如果编译失败，仔细阅读错误信息，修正代码后再次提交。最多重试 15 次。
5. 编译通过后，用自然语言（中文）向用户解释证明思路，引用你用到的关键引理与 tactic。
6. 如需查看代码中已有的声明或确认是否含 sorry，可调用 `get_proof_state`。

# 严格的规则（违反将导致证明无效）

- 禁止使用 `sorry` 或 `admit` 跳过任何子目标。
- 禁止修改用户给定的定义、公理、命题陈述。只能编写证明部分。
- 禁止声称"已证明"但实际未通过 `run_lean_check` 的代码。
- 每次提交给 `run_lean_check` 的代码必须是自洽的完整文件，包含所有必要 import。
- 不要在 Lean 代码中放入解释性注释代替真实证明。

# Lean4 写作约定

- 文件以 `import Mathlib` 起头（如果需要 mathlib）。
- 命名定理时使用 snake_case。
- 优先使用 `simp`、`ring`、`norm_num`、`decide` 等自动化 tactic。
- 复杂证明用 `calc` 或 `have` 分步。
- 数学符号使用 Unicode：`→ ≠ ≤ ≥ ∃ ∀ ∧ ∨ ¬ ↦`，不要用 ASCII 替代。

# 常用 tactic 速查

- `rfl`：自反性
- `rw [h]`：用等式 h 重写
- `simp`：化简
- `ring`：环等式
- `norm_num`：数值计算
- `decide`：可判定命题
- `induction n with | zero => _ | succ n ih => _`：自然数归纳
- `cases h with | inl h => _ | inr h => _`：分情况
- `have h : P := by ...`：引入中间结论
- `obtain ⟨h1, h2⟩ := h`：拆分与存在量词
- `apply`：反向推理
- `exact`：精确提供项
- `constructor`：拆分 ∧ ↔
- `rcases h with ⟨a, b, c⟩`：递归拆分
- `by_contra h`：反证法
- `use`：提供存在见证

# 输出格式

- 调用 `run_lean_check` 时，把完整 Lean4 代码作为 `lean_code` 参数传入。
- 最终回答用中文，结构如下：
  1. **证明思路**：2-4 句话概述证明策略。
  2. **关键引理**：列出用到的核心引理名称。
  3. **Lean4 代码**：用 ```lean4 代码块包裹最终通过的代码。
  4. **tactic 解释**：对非平凡的 tactic 调用给出简短说明。

# 限制

- 如果用户的问题超出 Lean4 + mathlib 的覆盖范围（如高阶范畴论、自定义代数结构），明确告知并建议替代方案。
- 如果重试 15 次仍无法通过编译，停止重试，向用户说明困难所在并请求澄清。
```

---

## 4. 前端代码

前端使用 React 18 + Vite + TypeScript，代码编辑器使用 CodeMirror 6（含 Lean 语法高亮），消息渲染使用 `react-markdown`。

### 4.1 `frontend/package.json`

```json
{
  "name": "deepseek-leanspark-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@codemirror/lang-lean": "^0.1.2",
    "@uiw/react-codemirror": "^4.23.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-markdown": "^9.0.1",
    "remark-gfm": "^4.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "typescript": "^5.5.3",
    "vite": "^5.4.0"
  }
}
```

> 注：`@codemirror/lang-lean` 在 npm 上由社区维护。若不存在则改用 `@codemirror/lang-javascript` 配合 `StreamLanguage.define` 自定义；M4 验收不依赖语法高亮正确性，仅依赖编辑器可输入与提交。

### 4.2 `frontend/vite.config.ts`

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
```

### 4.3 `frontend/tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "noEmit": true,
    "resolveJsonModule": true,
    "allowImportingTsExtensions": true
  },
  "include": ["src"]
}
```

### 4.4 `frontend/index.html`

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>DeepSeek-LeanSpark</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

### 4.5 `frontend/src/main.tsx`

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

### 4.6 `frontend/src/types.ts`

```typescript
export interface AgentEvent {
  kind: 'thinking' | 'tool_call' | 'tool_result' | 'answer' | 'error';
  content: string;
  tool_name?: string;
  tool_args?: unknown;
}

export interface ChatResponseDto {
  events: AgentEvent[];
  messages: unknown[];
}

export interface LeanResult {
  success: boolean;
  output: string;
  error: string | null;
  contains_sorry: boolean;
}

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  events?: AgentEvent[];
}
```

### 4.7 `frontend/src/hooks/useAgent.ts`

```typescript
import { useCallback, useState } from 'react';
import type { AgentEvent, ChatMessage, ChatResponseDto } from '../types';

interface UseAgentOptions {
  thinking?: boolean;
}

export function useAgent(options: UseAgentOptions = {}) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const send = useCallback(
    async (text: string) => {
      if (!text.trim() || isRunning) return;
      setIsRunning(true);
      setError(null);

      // 构造给后端的历史：把本地消息序列扁平化为 {role, content}
      const history = messages.map((m) => ({ role: m.role, content: m.content }));

      // 乐观追加用户消息
      const userMsg: ChatMessage = { role: 'user', content: text };
      setMessages((prev) => [...prev, userMsg]);

      try {
        const resp = await fetch('/api/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            message: text,
            history,
            thinking: options.thinking ?? false,
          }),
        });
        if (!resp.ok) {
          const errText = await resp.text();
          throw new Error(`HTTP ${resp.status}: ${errText}`);
        }
        const data: ChatResponseDto = await resp.json();

        // 从事件中抽取最终的 answer 作为 assistant 消息内容
        const answerEvent = [...data.events].reverse().find((e) => e.kind === 'answer');
        const assistantContent = answerEvent?.content ?? '(无回答)';
        const assistantMsg: ChatMessage = {
          role: 'assistant',
          content: assistantContent,
          events: data.events,
        };
        setMessages((prev) => [...prev, assistantMsg]);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        setMessages((prev) => [
          ...prev,
          { role: 'assistant', content: `错误: ${msg}` },
        ]);
      } finally {
        setIsRunning(false);
      }
    },
    [messages, isRunning, options.thinking],
  );

  const reset = useCallback(() => {
    setMessages([]);
    setError(null);
  }, []);

  return { messages, send, reset, isRunning, error };
}
```

### 4.8 `frontend/src/components/CodeEditor.tsx`

```tsx
import { useMemo } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { lean } from '@codemirror/lang-lean';

interface Props {
  value: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
}

export function CodeEditor({ value, onChange, readOnly }: Props) {
  const extensions = useMemo(() => (lean ? [lean()] : []), []);
  return (
    <CodeMirror
      value={value}
      height="auto"
      theme="light"
      readOnly={readOnly}
      extensions={extensions}
      onChange={onChange}
      basicSetup={{ lineNumbers: true, highlightActiveLine: !readOnly }}
    />
  );
}
```

### 4.9 `frontend/src/components/ProofState.tsx`

```tsx
import type { AgentEvent } from '../types';

interface Props {
  events: AgentEvent[];
}

export function ProofState({ events }: Props) {
  // 抽取最后一次 tool_result 中的 run_lean_check / get_proof_state 结果
  const lastLeanCheck = [...events]
    .reverse()
    .find((e) => e.kind === 'tool_result' && e.tool_name === 'run_lean_check');
  const lastProofState = [...events]
    .reverse()
    .find((e) => e.kind === 'tool_result' && e.tool_name === 'get_proof_state');

  return (
    <div className="proof-state">
      <h3>证明状态</h3>
      {lastLeanCheck && (
        <div>
          <h4>最近一次编译</h4>
          <pre>{lastLeanCheck.content}</pre>
        </div>
      )}
      {lastProofState && (
        <div>
          <h4>声明分析</h4>
          <pre>{lastProofState.content}</pre>
        </div>
      )}
      {!lastLeanCheck && !lastProofState && (
        <p className="muted">尚无证明状态信息。</p>
      )}
    </div>
  );
}
```

### 4.10 `frontend/src/components/ChatPanel.tsx`

```tsx
import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { AgentEvent, ChatMessage } from '../types';
import { ProofState } from './ProofState';

interface Props {
  messages: ChatMessage[];
  isRunning: boolean;
  onSend: (text: string) => void;
  onReset: () => void;
}

function EventItem({ event }: { event: AgentEvent }) {
  const palette: Record<AgentEvent['kind'], string> = {
    thinking: '#6b7280',
    tool_call: '#2563eb',
    tool_result: '#059669',
    answer: '#111827',
    error: '#dc2626',
  };
  const label: Record<AgentEvent['kind'], string> = {
    thinking: '思考',
    tool_call: '调用工具',
    tool_result: '工具结果',
    answer: '回答',
    error: '错误',
  };
  const color = palette[event.kind];
  return (
    <details
      style={{ borderLeft: `3px solid ${color}`, margin: '4px 0', padding: '4px 8px' }}
    >
      <summary style={{ color, cursor: 'pointer' }}>
        {label[event.kind]}
        {event.tool_name ? `: ${event.tool_name}` : ''}
      </summary>
      <pre style={{ whiteSpace: 'pre-wrap', fontSize: 12, marginTop: 4 }}>
        {event.content.length > 2000
          ? event.content.slice(0, 2000) + '\n...(已截断)'
          : event.content}
      </pre>
    </details>
  );
}

function MessageBubble({ msg }: { msg: ChatMessage }) {
  if (msg.role === 'user') {
    return (
      <div className="msg user">
        <strong>你</strong>
        <div>{msg.content}</div>
      </div>
    );
  }
  return (
    <div className="msg assistant">
      <strong>LeanSpark</strong>
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{msg.content}</ReactMarkdown>
      {msg.events && msg.events.length > 0 && (
        <div className="events">
          {msg.events.map((e, i) => (
            <EventItem key={i} event={e} />
          ))}
        </div>
      )}
    </div>
  );
}

export function ChatPanel({ messages, isRunning, onSend, onReset }: Props) {
  const [input, setInput] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;
    onSend(input);
    setInput('');
  };

  // 取最近一条 assistant 消息的事件作为 ProofState 显示
  const lastAssistant = [...messages].reverse().find((m) => m.role === 'assistant');
  const lastEvents = lastAssistant?.events ?? [];

  return (
    <div className="chat-panel" style={{ display: 'flex', gap: 16, height: '100%' }}>
      <div style={{ flex: 2, display: 'flex', flexDirection: 'column' }}>
        <div style={{ flex: 1, overflowY: 'auto', padding: 8 }}>
          {messages.map((m, i) => (
            <MessageBubble key={i} msg={m} />
          ))}
          {isRunning && <div className="muted">Agent 思考中...</div>}
        </div>
        <form onSubmit={handleSubmit} style={{ display: 'flex', gap: 8, padding: 8 }}>
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="输入你的数学命题或问题..."
            style={{ flex: 1, padding: 8 }}
            disabled={isRunning}
          />
          <button type="submit" disabled={isRunning || !input.trim()}>
            发送
          </button>
          <button type="button" onClick={onReset} disabled={isRunning}>
            清空
          </button>
        </form>
      </div>
      <div style={{ flex: 1, borderLeft: '1px solid #ddd', padding: 8, overflowY: 'auto' }}>
        <ProofState events={lastEvents} />
      </div>
    </div>
  );
}
```

### 4.11 `frontend/src/App.tsx`

```tsx
import { useAgent } from './hooks/useAgent';
import { ChatPanel } from './components/ChatPanel';
import { useState } from 'react';

export default function App() {
  const [thinking, setThinking] = useState(false);
  const { messages, send, reset, isRunning } = useAgent({ thinking });

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header
        style={{
          padding: 12,
          borderBottom: '1px solid #ddd',
          display: 'flex',
          alignItems: 'center',
          gap: 12,
        }}
      >
        <h1 style={{ margin: 0, fontSize: 18 }}>DeepSeek-LeanSpark</h1>
        <label style={{ fontSize: 13 }}>
          <input
            type="checkbox"
            checked={thinking}
            onChange={(e) => setThinking(e.target.checked)}
          />{' '}
          thinking 模式
        </label>
        <span style={{ color: '#888', fontSize: 12 }}>
          DeepSeek V4 + Lean4 辅助形式化证明
        </span>
      </header>
      <main style={{ flex: 1, overflow: 'hidden' }}>
        <ChatPanel
          messages={messages}
          isRunning={isRunning}
          onSend={send}
          onReset={reset}
        />
      </main>
    </div>
  );
}
```

---

## 5. 配置文件

> 路径说明：本节起所有非绝对路径均相对于 `DeepSeek-LeanSpark/`。

### 5.1 `.env.example`

```env
# DeepSeek API（必填）
DEEPSEEK_API_KEY=sk-your-api-key-here

# 模型选择：deepseek-v4-pro | deepseek-v4-flash
# flash 更便宜、更快；pro 推理更强。Phase 1 MVP 默认 flash。
DEEPSEEK_MODEL=deepseek-v4-flash

# Lean4 可执行文件路径。若已加入 PATH 直接用 "lean"。
# Windows 示例: C:\elan\bin\lean.exe
# Linux/macOS 示例: /usr/local/bin/lean
LEAN_BIN_PATH=lean

# 后端监听地址
LISTEN_ADDR=0.0.0.0:3000

# 日志级别：error|warn|info|debug|trace
RUST_LOG=info
```

### 5.2 `.gitignore`

```gitignore
# Rust
/target
**/*.rs.bk
Cargo.lock.bak

# 环境
.env
.env.local

# IDE
.vscode/
.idea/
*.swp

# 前端
frontend/node_modules/
frontend/dist/
frontend/.vite/

# 临时
*.log
leanspark_*.lean
```

### 5.3 `LICENSE`（MIT）

```text
MIT License

Copyright (c) 2026 DeepSeek-LeanSpark Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## 6. 文档

Phase 1 涉及两个 README：仓库根 `README.md` 做导航，项目级 `DeepSeek-LeanSpark/README.md` 给启动步骤。

### 6.1 仓库根 `README.md`

````markdown
# deepseek-leanspark

AI 辅助形式化数学工具：DeepSeek V4 + Lean4。

## 仓库结构

```
deepseek-leanspark/
├── docs/                    # 跨阶段文档
│   ├── phase1.md            # Phase 1 实现文档
│   ├── phase2.md            # （未来）
│   └── leanspark-guide.html # 项目知识基线
├── DeepSeek-LeanSpark/      # 项目主代码（Phase 1 实现，Phase 2 在此演进）
└── scripts/                 # CI/发布/安装脚本（未来）
```

## 阶段文档

| 阶段 | 文档 | 状态 |
|---|---|---|
| Phase 1 (MVP Web App) | [docs/phase1.md](./docs/phase1.md) | 进行中 |
| Phase 2 (Tauri 桌面应用) | docs/phase2.md | 规划中 |
| Phase 3 (DeepSeek-Prover 集成) | — | 规划中 |

## 快速开始

参见 [DeepSeek-LeanSpark/README.md](./DeepSeek-LeanSpark/README.md)。

## 项目知识基线

参见 [docs/leanspark-guide.html](./docs/leanspark-guide.html)。
````

### 6.2 项目级 `DeepSeek-LeanSpark/README.md`

````markdown
# DeepSeek-LeanSpark

AI 辅助形式化数学工具：DeepSeek V4 + Lean4。

> 以下所有命令均假定当前工作目录为 `DeepSeek-LeanSpark/`。从仓库根进入：`cd DeepSeek-LeanSpark`。

## 环境要求

- Rust 1.75+（推荐 stable）
- Node.js 18+
- Lean4（通过 [elan](https://github.com/leanprover/elan) 安装）
- DeepSeek API Key

## 安装 Lean4

```bash
# Linux / macOS
curl https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.sh -sSf | sh

# Windows PowerShell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.ps1" -OutFile elan-init.ps1
./elan-init.ps1
```

验证：

```bash
lean --version
```

## 启动后端

```bash
cp .env.example .env
# 编辑 .env 填入 DEEPSEEK_API_KEY

cargo run
```

后端默认监听 `http://localhost:3000`。

## 启动前端

```bash
cd frontend
npm install
npm run dev
```

前端默认监听 `http://localhost:5173`，自动代理 `/api` 到后端。

## 健康检查

```bash
curl http://localhost:3000/api/health
# => ok
```

## 直接调用 Lean 检查

```bash
curl -X POST http://localhost:3000/api/lean/check \
  -H "Content-Type: application/json" \
  -d '{"code":"theorem t : 1+1=2 := by rfl"}'
```

## API 一览

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/health` | 健康检查 |
| POST | `/api/chat` | Agent 主对话接口 |
| POST | `/api/lean/check` | 单独提交 Lean4 代码编译 |
| GET | `/api/tools` | 列出已注册工具 |
| GET | `/api/models` | 列出可用模型 |

### `/api/chat` 请求体

```json
{
  "message": "证明自然数加法交换律",
  "history": [],
  "thinking": false
}
```

### `/api/chat` 响应体

```json
{
  "events": [
    { "kind": "tool_call", "content": "调用工具: run_lean_check", "tool_name": "run_lean_check" },
    { "kind": "tool_result", "content": "{\"success\":true,...}", "tool_name": "run_lean_check" },
    { "kind": "answer", "content": "证明思路：..." }
  ],
  "messages": [/* 完整消息历史，可直接作为下次请求的 history */]
}
```

## 开发

```bash
# 后端热重载（需安装 cargo-watch）
cargo install cargo-watch
cargo watch -x run

# 前端开发服务器
cd frontend && npm run dev
```

## 生产构建

```bash
cd frontend && npm run build   # 产物在 frontend/dist/
cargo build --release          # 产物在 target/release/deepseek-leanspark
```

## 项目结构

见 [`../docs/phase1.md`](../docs/phase1.md)。
````

---

## 7. 运行与测试

### 7.1 启动顺序

1. `cd DeepSeek-LeanSpark`，复制 `.env.example` 为 `.env`，填入 `DEEPSEEK_API_KEY`，确认 `LEAN_BIN_PATH` 指向可执行的 `lean`。
2. 终端 A（在 `DeepSeek-LeanSpark/` 下）：`cargo run`（后端，端口 3000）。
3. 终端 B（在 `DeepSeek-LeanSpark/` 下）：`cd frontend && npm install && npm run dev`（前端，端口 5173）。
4. 浏览器打开 `http://localhost:5173`。

### 7.2 M1 验收脚本

```bash
# health
curl http://localhost:3000/api/health

# 工具列表
curl http://localhost:3000/api/tools
```

### 7.3 M2 验收脚本

```bash
# 通过
curl -X POST http://localhost:3000/api/lean/check \
  -H "Content-Type: application/json" \
  -d '{"code":"theorem t : 1+1=2 := by rfl"}'
# 期望 {"success":true,...}

# 失败
curl -X POST http://localhost:3000/api/lean/check \
  -H "Content-Type: application/json" \
  -d '{"code":"theorem t : 1+1=3 := by rfl"}'
# 期望 {"success":false,"error":"...type mismatch..."}

# 检测 sorry
curl -X POST http://localhost:3000/api/lean/check \
  -H "Content-Type: application/json" \
  -d '{"code":"theorem t : False := by sorry"}'
# 期望 contains_sorry=true
```

### 7.4 M3 验收脚本

```bash
curl -X POST http://localhost:3000/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"证明: 对任意自然数 n, n + 0 = n","history":[],"thinking":false}'
```

期望响应 `events` 中至少出现一次 `tool_call(run_lean_check)`，最终以 `kind: "answer"` 结尾，且 answer 内容含中文证明解释。

### 7.5 M4 验收

浏览器中：
1. 在输入框输入 `证明: 对任意自然数 n, n + 0 = n`。
2. 点击发送，应看到右侧 `证明状态` 面板显示最近的 `run_lean_check` 结果。
3. 左侧消息区中 LeanSpark 回复下方可展开 `思考 / 调用工具 / 工具结果 / 回答` 四类事件。
4. 勾选 `thinking 模式` 复选框后再次提问，应看到 `思考` 事件出现。

### 7.6 M5 验收

在干净的 Linux 机器上：

```bash
git clone <repo>
cd deepseek-leanspark/DeepSeek-LeanSpark
cp .env.example .env && $EDITOR .env
# 安装 Rust、Node、Lean4
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && sudo apt install -y nodejs
curl https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.sh -sSf | sh -s -- -y
cargo run &
cd frontend && npm install && npm run dev
```

应能完整复现 7.2~7.4 的验收。

---

## 8. 已知限制（Phase 1 范围内可接受）

| 项 | 限制 | 计划 |
|---|---|---|
| `lean` 单文件编译 | 不支持 lake 工程上下文，无法引用本地模块 | Phase 2 引入工作区概念 |
| `search_mathlib` | 仅内置 60 条常见引理索引 | Phase 3 接入 mathlib 全文搜索 |
| `get_proof_state` | 静态字符串解析，无实时 goal state | Phase 2 通过 LSP 提供实时状态 |
| 消息流 | 非流式，整轮响应完成后才返回 | Phase 2 加 SSE 流式 |
| 历史长度 | 无自动压缩，长对话会超 token 上限 | Phase 2 加滑动窗口 |
| thinking 触发 | 由前端 checkbox 手动开关 | Phase 2 由 Agent 自主决策 |
| 工具调用错误恢复 | 工具抛错直接回灌 JSON 给模型 | 已足够，必要时 Phase 2 加 retry 计数 |

---

## 9. 关键实现要点速查

> 下列文件链接均相对于 `DeepSeek-LeanSpark/`，例如 `src/deepseek/client.rs` 完整路径为 `DeepSeek-LeanSpark/src/deepseek/client.rs`。

1. **thinking 模式回传陷阱**：`ChatResponse::to_assistant_message()` 在 `reasoning_content` 存在时必须把它写回消息体，否则下一次 `chat_with_thinking` 调用会返回 400。这是 [client.rs](file:///d:/project/PROJECTS/deepseek-leanspark/DeepSeek-LeanSpark/src/deepseek/client.rs) 中 `to_assistant_message` 方法的关键逻辑。

2. **临时文件清理**：[runner.rs](file:///d:/project/PROJECTS/deepseek-leanspark/DeepSeek-LeanSpark/src/lean/runner.rs) 中即使 `lean` 启动失败也要删除临时文件，否则 `temp_dir` 会被 `leanspark_*.lean` 撑爆。

3. **sorry 检测**：在两个层面检测——`LeanRunner::run` 检测源码、`LeanCheckTool::call` 在工具结果中加 WARNING，让模型自己也看到。

4. **MAX_ITERATIONS=20**：[agent_loop.rs](file:///d:/project/PROJECTS/deepseek-leanspark/DeepSeek-LeanSpark/src/agent/agent_loop.rs) 中硬编码。System Prompt 要求模型最多重试 15 次，留 5 次余量给 search/get_proof_state 调用。

5. **CORS**：开发环境靠 Vite proxy，生产环境靠 `CorsLayer::permissive()`。如果前后端同源部署，CORS 层可移除。

6. **历史消息格式**：`/api/chat` 返回的 `messages` 字段含 system prompt 和完整对话历史（含 tool_calls / tool results）。前端下次请求时把它作为 `history` 传回，即可实现多轮对话。注意：前端要先剥掉 system prompt（在本实现中后端每次都会重新加 system prompt，前端传的 history 不应再含 system 消息——见 [agent_loop.rs](file:///d:/project/PROJECTS/deepseek-leanspark/DeepSeek-LeanSpark/src/agent/agent_loop.rs) 第 30-35 行）。

   实际上前端 `useAgent.ts` 只把 `messages.map(m => ({role, content}))` 传回，丢了 tool_calls 和 tool results。这意味着前端只保留自然语言对话历史，工具调用细节不进入下一轮——这是有意的，避免长对话中历史膨胀。**代价**：模型在第二轮对话中不知道第一轮的 Lean 代码。Phase 2 可改为完整传递。

7. **axum 0.8 的 `axum::serve`**：需要 `tokio::net::TcpListener`，不是 `std::net::TcpListener`。见 [lib.rs](file:///d:/project/PROJECTS/deepseek-leanspark/DeepSeek-LeanSpark/src/lib.rs)。

8. **`include_str!`**：System Prompt 在编译期内联，运行时无需读文件，但修改 `prompts/agent-prompt.md` 后必须 `cargo build`。

---

文档结束。
