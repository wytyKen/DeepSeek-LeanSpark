# 依赖声明

本文档列出 DeepSeek-LeanSpark 项目所有运行时与开发时依赖及其许可证。Phase 1 范围。

## 后端 Rust 依赖（`DeepSeek-LeanSpark/Cargo.toml`）

| Crate | 版本 | 用途 | 许可证 |
|---|---|---|---|
| `tokio` | 1 | 异步运行时 | MIT |
| `axum` | 0.8 | Web 框架 | MIT |
| `tower` | 0.5 | 中间件抽象 | MIT |
| `tower-http` | 0.6 | HTTP 中间件（CORS、Trace、ServeDir） | MIT |
| `reqwest` | 0.12 | HTTP 客户端（调用 DeepSeek API） | MIT OR Apache-2.0 |
| `serde` | 1 | 序列化框架 | MIT OR Apache-2.0 |
| `serde_json` | 1 | JSON 序列化 | MIT OR Apache-2.0 |
| `tracing` | 0.1 | 结构化日志 | MIT |
| `tracing-subscriber` | 0.3 | 日志订阅器 | MIT |
| `uuid` | 1 | 唯一 ID 生成（临时文件名） | MIT OR Apache-2.0 |
| `dotenvy` | 0.15 | `.env` 文件加载 | MIT |
| `thiserror` | 1 | 派生 Error trait | MIT OR Apache-2.0 |
| `anyhow` | 1 | 错误聚合 | MIT OR Apache-2.0 |
| `async-trait` | 0.1 | async trait 支持 | MIT OR Apache-2.0 |
| `once_cell` | 1 | 全局静态初始化 | MIT OR Apache-2.0 |

### 开发依赖

| Crate | 版本 | 用途 | 许可证 |
|---|---|---|---|
| `reqwest` (blocking) | 0.12 | 集成测试 HTTP 客户端 | MIT OR Apache-2.0 |

### 间接依赖

完整间接依赖列表由 `Cargo.lock` 维护，可通过 `cargo license` 命令导出。所有间接依赖均兼容 MIT 许可证。

## 前端 Node 依赖（`DeepSeek-LeanSpark/frontend/package.json`）

| 包 | 版本 | 用途 | 许可证 |
|---|---|---|---|
| `react` | ^18.3 | UI 框架 | MIT |
| `react-dom` | ^18.3 | React DOM 渲染 | MIT |
| `react-markdown` | ^9.0 | Markdown 渲染 | MIT |
| `remark-gfm` | ^4.0 | GitHub Flavored Markdown 支持 | MIT |
| `@uiw/react-codemirror` | ^4.23 | CodeMirror 6 React 封装 | MIT |
| `@codemirror/lang-lean` | ^0.1.2 | Lean 语法高亮 | MIT |
| `@types/react` | ^18.3 | React 类型定义 | MIT |
| `@types/react-dom` | ^18.3 | React DOM 类型定义 | MIT |
| `@vitejs/plugin-react` | ^4.3 | Vite React 插件 | MIT |
| `typescript` | ^5.5 | TypeScript 编译器 | Apache-2.0 |
| `vite` | ^5.4 | 构建工具 | MIT |

## 外部运行时依赖

| 依赖 | 版本 | 用途 | 安装方式 |
|---|---|---|---|
| Rust | 1.75+ | 编译后端 | https://rustup.rs |
| Node.js | 18+ | 构建前端 | https://nodejs.org |
| Lean4 | 4.x | 形式化证明引擎 | https://github.com/leanprover/elan |
| DeepSeek API | — | LLM 后端 | https://platform.deepseek.com |

## 许可证兼容性

本项目采用 MIT 许可证。所有上述依赖的许可证均与 MIT 兼容（MIT、Apache-2.0）。

## 更新流程

依赖版本由 `cargo update` 与 `npm update` 维护。重大升级需通过 PR 评审，并在 `CHANGELOG.md` 中记录。

## SBOM 生成

发布时可通过以下命令生成软件物料清单：

```bash
# Rust SBOM
cargo install cargo-cyclonedx
cargo cyclonedx -f json --format pretty

# Node SBOM
npm install -g @cyclonedx/cyclonedx-npm
cyclonedx-npm --output-file sbom-frontend.json --output-format JSON
```
