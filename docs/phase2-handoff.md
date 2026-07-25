# Phase 2 交接文档

> 本文档供 Phase 2 执行者（新对话中的 AI 模型）快速接手项目。包含：项目当前状态、已完成内容、Phase 2 待办、必须遵守的硬约束、易被忽视的重要细节、已修复 bug 清单。

---

## 0. 快速定位

| 项 | 值 |
|---|---|
| 仓库根绝对路径 | `d:\project\PROJECTS\deepseek-leanspark` |
| GitHub 仓库 | `https://github.com/wytyKen/DeepSeek-LeanSpark` |
| 主代码目录 | `DeepSeek-LeanSpark/` |
| Tauri 壳目录 | `DeepSeek-LeanSpark/src-tauri/` |
| 前端目录 | `DeepSeek-LeanSpark/frontend/` |
| API Key 配置 | `DeepSeek-LeanSpark/.env`（变量 `DEEPSEEK_API_KEY`，已 gitignore） |
| 系统提示词 | `DeepSeek-LeanSpark/prompts/agent-prompt.md` |
| 当前 git 分支 | `main`（跟踪 `origin/main`） |
| Phase 1 最后提交 | `2629e62`（Fix Tauri desktop shell: try_init） |
| 未提交改动 | 27 个文件（19 modified + 7 untracked + 1 deleted）：本轮 P0/P1 测试代码、bug 修复、README 重写、交接文档 |

---

## 1. 项目当前状态

### 1.1 Phase 1（Web MVP）—— 已完成 ✅

- 后端 Rust + Axum，端口 3000
- 前端 React + Vite，端口 5173
- Agent 工具调用循环（5 个工具：run_lean_check / read_file / write_file / search / proof_state）
- 工作区管理、证明依赖图、LaTeX 渲染、文件树
- DeepSeek Chat / Reasoner 集成（含 thinking 模式）

### 1.2 Phase 1.5（UI 增强）—— 已完成 ✅

- 聊天面板 Trae Work 风格
- 右侧栏三 Tab（资源管理器 / 证明依赖图 / 公式）
- KaTeX 公式渲染
- React Flow 依赖图

### 1.3 测试体系 —— 已完成 ✅

| 类型 | 数量 | 位置 | 状态 |
|---|---|---|---|
| Rust 单元测试 | 111 | `src/**/*.rs` 内联 `#[cfg(test)]` | 全通过 |
| 前端组件测试 | 49 | `frontend/src/**/*.test.tsx` | 全通过 |
| Rust 集成测试 | 3 文件 | `tests/*.rs` | 需后端运行 |

### 1.4 Phase 2（Tauri 桌面应用）—— 部分完成 ⚠️

**已完成**：
- `src-tauri/src/main.rs`：Tauri 主进程内嵌 axum 后端（tokio runtime 承载），注册 dialog/fs/shell 插件
- `src-tauri/tauri.conf.json`：窗口配置（1400×900）、CSP、bundle 配置、图标
- `src-tauri/Cargo.toml`：依赖 tauri 2.x + 三个插件
- `frontend/src/lib/tauri.ts`：环境检测（`window.__TAURI_INTERNALS__`）+ 原生文件对话框（带 Web 降级）
- `src-tauri/icons/`：齐全（含 Windows `icon.ico`，必需）
- `run-tauri.bat`：Windows 启动脚本（纯 ASCII，goto labels）

**待完成（Phase 2 核心任务）**：
1. 验证 `cargo tauri build` 能成功产出多平台安装包
2. 添加 `.github/workflows/release.yml`（tag 触发，多平台构建，上传 GitHub Release）
3. Tauri 应用内 API Key 设置 UI（替代手动编辑 .env）
4. Lean4 打包策略决策与实现
5. 创建 `docs/phase2.md` 设计文档
6. 更新所有文档反映 Phase 2 完成
7. 提交所有未提交改动并推送 GitHub

---

## 2. 必须遵守的硬约束（违反会导致构建/CI 失败）

### 2.1 仓库结构
- GitHub 仓库必须命名为 `DeepSeek-LeanSpark`，设为 **public**
- 仓库根必须包含：`DeepSeek-LeanSpark/` 目录、`docs/`、`.github/`、根级脚本（`run-web.bat` / `run-tauri.bat`）
- `.gitignore` 必须排除：`.env`、`target/`、`node_modules/`、`dist/`、`src-tauri/target/`、`src-tauri/gen/`

### 2.2 Windows 兼容性
- **批处理脚本必须纯 ASCII**：`run-web.bat` / `run-tauri.bat` 中不能出现中文字符，否则 Windows cmd 编码错误导致崩溃
- `run-tauri.bat` 必须用 **goto labels** 而非嵌套 if 块（带括号的嵌套 if 在 cmd 中易出错）
- `run-web.bat` 必须含 `--open` flag（自动打开浏览器）
- Tauri CLI 安装命令：`cargo install tauri-cli`（**不带 `--version` 参数**，否则 semver 解析失败）

### 2.3 CI/CD
- GitHub Actions 必须用 **Node.js 22+**（Node 20 已废弃，会失败）
- 必须用 `actions/checkout@v5` + `actions/setup-node@v5`
- Lean4 安装必须用 **官方 elan installer**（`curl + elan-init.sh`），不要用 `lean-action`（其 `auto-config: true` 在 ubuntu-latest 上不可靠）

### 2.4 Rust
- **不要在 lib.rs 或 main.rs 用 `.init()` 初始化 tracing_subscriber**：多次初始化会 panic，必须用 `.try_init()` 并忽略返回值
- Tauri build 在 Windows **必须**有 `src-tauri/icons/icon.ico`，否则失败

### 2.5 API Key
- 存储位置：`DeepSeek-LeanSpark/.env`，变量名 `DEEPSEEK_API_KEY`
- `.env` 不热加载，修改后必须重启后端（Web: `run-web.bat`，Tauri: `run-tauri.bat`）
- `.env` 必须在 `.gitignore` 中，**绝不能提交到 GitHub**

---

## 3. 关键技术设计决策（易被忽视）

### 3.1 lib + bin 双产物
- `src/lib.rs` 导出 `run()` 异步函数，供 Tauri 壳复用
- `src/main.rs` 是 Web 形态独立入口，直接 `tokio::main` 调 `run()`
- `src-tauri/src/main.rs` 在 Tauri 主进程内用 tokio runtime 调 `deepseek_leanspark::run()`
- **三处入口共享同一套后端代码**，修改后端逻辑时三处都会受影响

### 3.1b lib.rs 启动 panic 风险（Phase 2 必须处理）
- `src/lib.rs` 第 40 行：`std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set")`
- **当前行为**：.env 缺失或未设 key 时，后端启动直接 panic
- **Phase 2 影响**：Tauri 应用启动时若用户未配置 key，整个应用崩溃，无法进入设置界面
- **必须改为**：延迟初始化或优雅降级——启动时不读 key，首次调用 LLM 时才校验，或提供 UI 让用户输入后热重建 `DeepSeekClient`

### 3.1c 监听地址差异
- Web 形态（`src/lib.rs` 第 43 行）：默认 `0.0.0.0:3000`（**外网可访问**）
- Tauri 形态（`src-tauri/src/main.rs` 第 37-40 行）：强制 `127.0.0.1:3000`（**仅本机**）
- 两者通过环境变量 `LISTEN_ADDR` 覆盖
- **安全提示**：Web 形态部署时务必用反向代理或防火墙限制 3000 端口

### 3.2 ChatClient trait 依赖注入
- 定义在 `src/deepseek/mod.rs`：`pub trait ChatClient: Send + Sync`
- `AgentLoop` 依赖 `Arc<dyn ChatClient>` 而非具体 `DeepSeekClient`
- **目的**：单元测试可注入 `MockChatClient` 模拟 LLM 响应队列
- 修改 agent_loop 时不要破坏此抽象

### 3.3 Tauri 主进程内嵌 axum
- `src-tauri/src/main.rs` 用 `tokio::runtime::Runtime::new()` 创建 runtime，`spawn` 后端 `run()`
- runtime 存进 `app.manage()` 防止销毁
- 后端固定监听 `127.0.0.1:3000`（仅本机，避免外网暴露）
- 前端 Webview 通过 `http://localhost:3000/api/*` 调用后端

### 3.4 前端环境检测
- `frontend/src/lib/tauri.ts` 的 `isTauri()` 通过 `window.__TAURI_INTERNALS__` 判断
- `pickDirectory()` 在 Tauri 环境调原生对话框，Web 环境降级为 `window.prompt`
- **双形态共用同一套前端代码**，无需分两套构建配置

### 3.5 CSP 限制
- `tauri.conf.json` 的 CSP：`connect-src 'self' http://localhost:3000 http://127.0.0.1:3000`
- **扩展 API 端点或端口时必须同步更新 CSP**，否则前端请求被拦截

### 3.6 端口固定
- 后端：`127.0.0.1:3000`
- 前端 dev：`5173`
- CSP、vite 代理、tauri.conf.json 都硬编码了这两个端口，**不要随意改**

### 3.7 工具注册机制
- `ToolRegistry::new_with_workspace(lean, workspace)` 注册工作区相关工具（read_file/write_file 延迟注册）
- 工具通过 `spec()` 输出 OpenAI function-calling 格式 schema
- `agent_loop.rs` 调 `tools.dispatch(name, args)` 执行

### 3.8 Lean4 路径配置（Phase 2 打包关键）
- `src/lib.rs` 第 42 行：`std::env::var("LEAN_BIN_PATH").unwrap_or_else(|_| "lean".to_string())`
- **默认从 PATH 查找 `lean`**，可通过环境变量 `LEAN_BIN_PATH` 指定绝对路径
- Phase 2 打包策略（方案 B 引导安装 elan）需在应用启动时检测 `lean --version`，失败则引导用户
- `LeanRunner::new(lean_bin_path)` 接受路径字符串，运行时 `Command::new(&self.lean_bin)`

### 3.9 默认模型名
- `src/lib.rs` 第 41 行：`std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string())`
- 默认模型 `deepseek-v4-flash`，可通过环境变量 `DEEPSEEK_MODEL` 覆盖
- 实际可用模型见 DeepSeek API 文档（deepseek-chat / deepseek-reasoner）

### 3.10 Tauri 构建配置细节
- `src-tauri/Cargo.toml` 的 `rust-version = "1.77"`（比主 crate 1.75 高）
- `src-tauri/Cargo.toml` 的 `default = ["custom-protocol"]` feature 是 Tauri 生产构建必需
- `src-tauri/Cargo.toml` 通过 `deepseek-leanspark = { path = ".." }` 复用主 crate
- `tauri.conf.json` 的 `frontendDist: "../frontend/dist"` 指向前端构建产物
- `beforeBuildCommand: "npm --prefix ../frontend run build"` 在 Tauri build 前自动构建前端

---

## 4. 已修复的 Bug（避免回归）

| Bug | 文件 | 修复方式 | 回归信号 |
|---|---|---|---|
| Windows UNC 前缀路径被误拒 | `src/workspace/paths.rs` `validate_nonexistent_path` | 先 `strip_prefix(canonical_root)` 再处理相对段 | write_file 创建嵌套父目录不存在的文件失败 |
| lean IO 错误时 contains_sorry 硬编码 false | `src/lean/runner.rs` | 改为 `code.contains("sorry") \|\| code.contains("admit")` | lean 未安装时含 sorry 代码逃过安全检查 |
| tracing_subscriber 重复初始化 panic | `src/lib.rs` / `src-tauri/src/main.rs` | 用 `.try_init()` 替代 `.init()` | Tauri 启动 panic |
| CopyButton aria-label 静态 | `frontend/src/components/common/CopyButton.tsx` | 动态绑定 `aria-label={displayLabel}` | 屏幕阅读器读错状态 |
| FileTree 测试图标多元素匹配 | `frontend/src/components/sidebar/FileTree.test.tsx` | 用 `getAllByText` + 长度断言 | 测试失败 |
| EventCollapse 文本冲突 | `frontend/src/components/chat/EventCollapse.test.tsx` | 用 `.event-body` 选择器 | 测试失败 |
| setup.ts 缺 vi 导入 | `frontend/src/test/setup.ts` | 加 `import { vi } from 'vitest'` | `npm run build` 失败 |

---

## 5. 测试执行命令

```bash
# Rust 单元测试（无需后端运行，111 个）
cargo test --lib --manifest-path DeepSeek-LeanSpark/Cargo.toml

# 前端组件测试（49 个）
npm test --prefix DeepSeek-LeanSpark/frontend -- --run

# Rust 集成测试（需先启动后端）
cd DeepSeek-LeanSpark && cargo run   # 终端 1
cd DeepSeek-LeanSpark && cargo test --test api_smoke   # 终端 2

# 格式化与 lint
cargo fmt --manifest-path DeepSeek-LeanSpark/Cargo.toml --check
cargo clippy --manifest-path DeepSeek-LeanSpark/Cargo.toml --all-targets -- -D warnings
```

---

## 6. Phase 2 执行计划（建议顺序）

### 6.1 优先级 P0（核心交付）

1. **提交未提交改动**：本轮 P0/P1 测试代码 + bug 修复 + README 重写 + 交接文档（27 个文件），先 commit 推送
   - 建议拆为 2-3 个 commit：① test+fix（测试代码+bug修复）② docs（README+交接文档）③ chore（删除重复 CodeEditor.tsx）
   - 或单个 commit：`test: add P0/P1 unit tests, fix critical bugs, rewrite README, add Phase 2 handoff doc`
2. **修复 lib.rs 启动 panic 风险**（详见 3.1b）：把第 40 行 `expect` 改为延迟初始化，让应用在未配置 key 时能启动显示设置界面
3. **验证 `cargo tauri build`**：在 Windows 本地跑一次，确认能产出 `.msi` / `.exe`
4. **Tauri API Key UI**：在应用内提供设置界面，持久化到本地（避免用户编辑 .env）；用户输入后调 `DeepSeekClient::new` 重建客户端
5. **Lean4 打包策略**：建议方案 B（首次启动引导安装 elan），因为方案 A（打包 lean 二进制）体积过大且平台差异大

### 6.2 优先级 P1（分发自动化）

5. **GitHub Actions release workflow**：`.github/workflows/release.yml`，tag 触发，多平台构建（windows-latest / macos-latest / ubuntu-latest），上传 Release assets
6. **更新 `tauri.conf.json`**：版本号、bundle 配置、可能需要 `externalBin`（若选方案 A）

### 6.3 优先级 P2（文档完善）

7. **创建 `docs/phase2.md`**：设计文档，记录决策、架构、使用方式
8. **更新根 `README.md`**：把 Phase 2 状态从"待完成"改为"已完成"，更新使用方式
9. **更新 `DeepSeek-LeanSpark/README.md`**：补充 Tauri 桌面形态的详细使用说明
10. **更新 `docs/phase1.5-design.md`**：添加指向 phase2.md 的链接

### 6.4 收尾

11. **全量测试**：`cargo test --lib` + `npm test -- --run` + `cargo clippy` + `cargo fmt --check`
12. **提交并推送**：`git add` + `git commit` + `git push`
13. **打 tag 触发 release**：`git tag v0.2.0` + `git push origin v0.2.0`

---

## 7. 易被忽视的重要细节清单

1. **`run-tauri.bat` 是纯 ASCII**：修改时不要加中文注释或中文 echo
2. **`src-tauri/gen/` 从未被 git 跟踪**：虽然目录存在，但 .gitignore 在首次 commit 前已生效，无需 `git rm --cached`
3. **`tests/api_smoke.rs` 需要后端运行**：不是单元测试，`cargo test --lib` 不会跑它
4. **lean/runner.rs 测试用全局 Mutex 串行化**：避免临时文件 race condition，修改测试时不要移除 `TEST_LOCK`
5. **agent_loop 测试用 MockChatClient**：响应队列耗尽会 panic，添加测试用例时确保队列长度匹配
6. **前端测试 `setup.ts` 必须导入选 vi**：否则 `npm run build` 失败（不仅是测试，build 也走 ts 检查）
7. **CSP 限制 connect-src**：只允许 localhost:3000，扩展后端端口或加外部 API 必须同步更新
8. **Tauri 主进程 tracing 用 try_init()**：不要改成 init()
9. **API Key 变更需重启**：.env 不热加载，Phase 2 若加 UI 设置，需调用 `DeepSeekClient::new` 重建客户端
10. **仓库根的 README 是面向用户的入口**：不要写"参见子 README"，要直接写明项目是什么、怎么用
11. **GitHub 仓库地址**：`https://github.com/wytyKen/DeepSeek-LeanSpark`（组织名 `wytyKen`，根 README 中已替换）
12. **Lean4 路径配置**：环境变量 `LEAN_BIN_PATH`，默认 "lean"（从 PATH 查找），详见交接文档 3.8
13. **`Cargo.lock` 已提交**：这是应用项目（非库），应提交 lock 文件确保可复现构建
14. **`run-web.bat` 和 `run-tauri.bat` 在仓库根**：不在 `DeepSeek-LeanSpark/` 内，因为用户克隆后第一眼看到的是仓库根
15. **lib.rs 启动 panic 风险**：第 40 行 `expect("DEEPSEEK_API_KEY must be set")`，Phase 2 添加 UI 前必须改为延迟初始化，详见 3.1b
16. **Web 形态监听 0.0.0.0:3000**：外网可访问，Tauri 形态强制 127.0.0.1:3000，详见 3.1c
17. **src-tauri rust-version 1.77**：比主 crate 1.75 高，CI 需用 1.77+
18. **custom-protocol feature**：src-tauri/Cargo.toml 的 default feature，Tauri 生产构建必需，不要移除
19. **ci.yml 已正确配置**：Node 22 + actions/checkout@v5 + setup-node@v5 + elan installer，符合硬约束，release.yml 可参考其结构
20. **GitHub 仓库当前为 public**：已符合硬约束，无需额外设置
21. **ci.yml 前端 job 缺 `npm test`**：当前 frontend job 只跑 `npm run build`，**没跑 49 个前端组件测试**。Phase 2 应在 `npm run build` 前加一步 `npm test`，否则前端测试回归不会被 CI 发现
22. **.env.example 的 LISTEN_ADDR=0.0.0.0:3000**：这是 Web 形态默认值（外网可访问），.env.example 里缺安全警告注释。Phase 2 建议加注释说明生产部署应改为 127.0.0.1 或用反向代理
23. **package.json 的 test script 是 `vitest run`**：`npm test` 会跑 vitest（非 watch 模式），CI 里可直接用 `npm test`

---

## 8. 项目知识基线

更详细的项目背景、设计思路、术语规范，参见：
- [docs/leanspark-guide.html](./leanspark-guide.html) —— 项目知识基线
- [docs/phase1.md](./phase1.md) —— Phase 1 设计文档
- [docs/phase1.5-design.md](./phase1.5-design.md) —— Phase 1.5 增强设计
- [DeepSeek-LeanSpark/README.md](../DeepSeek-LeanSpark/README.md) —— Phase 1 完整实现说明

---

## 9. 交接确认

- ✅ 所有测试通过（111 Rust + 49 前端）
- ✅ `cargo fmt --check` 通过
- ✅ `cargo clippy -- -D warnings` 通过
- ✅ `npm run build` 通过
- ⚠️ 有 27 个文件未提交（19 modified + 7 untracked + 1 deleted），Phase 2 执行者需先提交（或交接者现在提交）
- ✅ Tauri 基础框架可用（`cargo tauri dev` 可启动）
- ✅ GitHub 仓库已存在且为 public：`https://github.com/wytyKen/DeepSeek-LeanSpark`
- ✅ CI（ci.yml）基本配置正确并通过（Node 22 + checkout@v5 + elan）
- ✅ `.env` 已 gitignore，`.env` 文件存在但不会被提交
- ✅ `Cargo.lock` 已跟踪
- ❌ `cargo tauri build` 未验证（Phase 2 任务）
- ❌ Release workflow 不存在（Phase 2 任务）
- ❌ `docs/phase2.md` 不存在（Phase 2 任务）
- ❌ lib.rs 启动 panic 风险未修复（Phase 2 添加 UI 前必须处理，详见 3.1b）
- ❌ ci.yml 前端 job 缺 `npm test` 步骤（Phase 2 应补上，详见第 7 条第 21 点）
- ❌ .env.example 的 LISTEN_ADDR 缺安全警告注释（Phase 2 建议补充，详见第 7 条第 22 点）
