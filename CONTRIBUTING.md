# 贡献指南

感谢你对 DeepSeek-LeanSpark 项目的兴趣！本文档描述贡献流程。

## 开发环境搭建

参见 [`DeepSeek-LeanSpark/README.md`](./DeepSeek-LeanSpark/README.md) 的"启动后端"与"启动前端"章节。开发前确保以下工具可用：

- Rust 1.75+（`rustc --version`）
- Node.js 18+（`node --version`）
- Lean4（`lean --version`）
- DeepSeek API Key（写入 `DeepSeek-LeanSpark/.env`）

## 开发工作流

### 1. Fork 与 Clone

```bash
git clone https://github.com/<your-username>/deepseek-leanspark.git
cd deepseek-leanspark
git remote add upstream https://github.com/<original>/deepseek-leanspark.git
```

### 2. 创建分支

```bash
git checkout -b feature/your-feature-name
# 或
git checkout -b fix/issue-123
```

分支命名约定：
- `feature/*`：新功能
- `fix/*`：bug 修复
- `docs/*`：文档改进
- `refactor/*`：代码重构
- `test/*`：测试补充

### 3. 编码

#### Rust 代码规范

- 使用 `cargo fmt` 格式化
- 使用 `cargo clippy -- -D warnings` 通过 lint 检查
- 公共 API 必须有文档注释 `///`
- 错误处理统一使用 `anyhow::Result`，自定义错误类型用 `thiserror`
- 异步代码用 `tokio` 运行时
- 提交前 `cargo test` 通过

#### TypeScript/React 代码规范

- 使用 2 空格缩进
- 使用单引号字符串
- 使用 `const` 优先，`let` 次之，禁用 `var`
- 组件用函数组件 + Hooks
- 提交前 `npm run build` 通过（含 `tsc` 类型检查）

#### Prompt 修改

修改 `DeepSeek-LeanSpark/prompts/agent-prompt.md` 后必须 `cargo build` 重新编译（Prompt 通过 `include_str!` 编译期内联）。

### 4. 提交

遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

```
<type>(<scope>): <subject>

<body>

<footer>
```

type 取值：
- `feat`：新功能
- `fix`：bug 修复
- `docs`：文档
- `style`：格式（不影响代码逻辑）
- `refactor`：重构
- `test`：测试
- `chore`：构建、依赖等杂项

scope 示例：`backend`、`frontend`、`agent`、`lean`、`tools`、`prompt`、`docs`

示例：
```
feat(agent): 增加 MAX_ITERATIONS 配置项

将硬编码的 20 改为可配置，通过环境变量 AGENT_MAX_ITERATIONS 控制。
默认值保持 20。

Closes #42
```

### 5. Push 与 PR

```bash
git push origin feature/your-feature-name
```

在 GitHub 上发起 Pull Request 到 `main` 分支。PR 描述需包含：
- 改动摘要
- 关联 Issue（如有）
- 测试方式
- Breaking changes（如有）

## PR 评审标准

- [ ] 代码通过 `cargo fmt` / `cargo clippy` / `cargo test`
- [ ] 代码通过 `npm run build`
- [ ] 新功能有对应测试
- [ ] 文档已更新（README、phase1.md 等）
- [ ] 提交信息符合 Conventional Commits
- [ ] 不引入新的直接依赖（除非必要，需在 PR 描述中说明）

## 项目结构

参见 [`docs/phase1.md`](./docs/phase1.md) 第 1 章。

## 报告 Bug

通过 GitHub Issues 报告 Bug，模板见 [`.github/ISSUE_TEMPLATE/bug_report.md`](./.github/ISSUE_TEMPLATE/bug_report.md)。

## 行为准则

请保持尊重与专业。攻击性、歧视性言论不被容忍。
