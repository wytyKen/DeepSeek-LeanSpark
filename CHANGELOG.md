# 变更日志

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 格式，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Added
- Phase 1 MVP 实现：
  - 后端 axum server（health / chat / lean/check / tools / models 路由）
  - DeepSeek V4 客户端（普通模式 + thinking 模式）
  - Lean4 子进程编译器集成
  - Agent 循环（最多 20 次迭代，含 tool_calls / stop 处理）
  - 3 个工具：`run_lean_check` / `search_mathlib` / `get_proof_state`
  - System Prompt（禁止 sorry、强制 Lean4 写作约定）
  - React + Vite 前端（对话面板、CodeMirror 编辑器、证明状态面板）
  - 端到端烟雾测试（`tests/api_smoke.rs`）
- 仓库根 README、DEPENDENCIES、CONTRIBUTING、SECURITY 文档
- Phase 1 实现文档（`docs/phase1.md`）
- 项目知识基线文档（`docs/leanspark-guide.html`）

### Security
- sorry / admit 双层检测（LeanRunner + LeanCheckTool WARNING）
- `.env` 加入 `.gitignore`
- API Key 通过后端代理，前端不接触

## 版本里程碑计划

- `0.1.0` — Phase 1 M5 发布（待发布）
- `0.2.0` — Phase 2 Tauri 桌面应用（规划中）
- `0.3.0` — Phase 3 DeepSeek-Prover V2 集成（规划中）

## 版本号规则

- Phase 1 期间：`0.1.x`，每完成一个里程碑可发 patch
- Phase 2 开始：`0.2.x`
- 第一个稳定版：`1.0.0`（Phase 3 完成后）
