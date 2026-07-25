# Phase 2 设计文档：原生桌面分发与运行时配置

> 本文档记录 DeepSeek-LeanSpark Phase 2 的架构决策、实现细节与发布流程。
> Phase 2 的目标是把项目从「源码运行」升级为「可分发安装包」，让终端用户无需安装 Rust/Node.js 工具链即可使用。
>
> 上游设计输入：[phase2-handoff.md](./phase2-handoff.md)（交接文档，9 章 23 条细节）
> 前序阶段：[phase1.md](./phase1.md)（核心实现）、[phase1.5-design.md](./phase1.5-design.md)（Tauri 集成与右侧栏）

## 1. 阶段目标

| 维度 | Phase 1.5 状态 | Phase 2 目标 | 完成情况 |
|---|---|---|---|
| 桌面打包 | `cargo tauri dev` 可用 | `cargo tauri build` 产出可分发安装包 | ✅ |
| API Key 配置 | 手动编辑 `.env` | 应用内 UI 设置，运行时注入 | ✅ |
| Lean4 依赖 | 用户自行安装，应用启动假设已就绪 | 启动时检测，未安装时弹引导 Modal | ✅ |
| 分发渠道 | 无 | GitHub Releases 三平台自动构建 | ✅ |
| 文档 | Phase 1.5 设计文档 | Phase 2 设计文档 + README 更新 | ✅ |

## 2. 关键架构决策

### 2.1 API Key 延迟初始化（解决启动 panic）

**背景**：Phase 1.5 的 `src/lib.rs` 第 40 行使用 `expect("DEEPSEEK_API_KEY must be set")`，未配置 key 时 Tauri 应用启动直接 panic，用户无法进入设置界面——这是 Phase 2 必须首先修复的关键风险。

**方案**：引入 `SharedChatClient` 包装器，把 `Arc<DeepSeekClient>` 替换为 `Arc<SharedChatClient>`：

```rust
pub struct SharedChatClient {
    inner: Arc<RwLock<Option<DeepSeekClient>>>,
}
```

- 启动时 `api_key = std::env::var("DEEPSEEK_API_KEY").ok()`，未配置则 `SharedChatClient::new(None)`
- `ChatClient` trait 实现中：未配置时 chat 调用返回友好错误（"DeepSeek API Key 未配置..."），不 panic
- 用户通过 UI 设置 key 后调 `replace_client` 注入真实客户端
- 实现 `ChatClient` trait，`AgentLoop` 依赖 `Arc<dyn ChatClient>`，无需修改 AgentLoop

**关键细节**：
- `RwLockReadGuard` 不是 `Send`，不能跨 `.await` 持有。实现中先 clone 出 `DeepSeekClient`（`DeepSeekClient` 是 `Clone`），drop guard，再调用 chat
- `is_configured()` / `model()` 方法供 API 查询状态
- `model()` 返回 `String` 而非 `&str`，避免持有 RwLock 读锁

**持久化策略**：
- 开发者：通过 `.env` 配置（`DEEPSEEK_API_KEY`），后端启动时 `dotenvy::dotenv()` 自动加载
- 终端用户：每次启动应用时通过 UI 输入（不持久化，重启后需重新配置）
- API Key 本身**不存 localStorage**（避免明文泄漏），这是安全与便利的折中

### 2.2 Lean4 打包策略：方案 B（首次启动引导安装）

**决策**：不打包 Lean4 二进制到安装包，采用"首次启动引导安装 elan"方案。

**理由**：
- Lean4 工具链体积大（数百 MB），打包会显著增大安装包
- 平台差异大（Windows/macOS/Linux 二进制不通用），需为每个平台单独打包
- 版本管理复杂，用户可能需要不同 Lean4 版本
- elan 已是 Lean4 官方推荐的版本管理器，用户安装一次即可管理多版本

**实现**：
- 后端新增 `LeanRunner::check_version()` 方法，调用 `lean --version` 检测可用性
- 后端新增 `/api/lean/check-install` GET 接口，返回 `{ installed, version, lean_bin, install_guide }`
- 前端新增 `useLeanInstall` hook，启动时自动调用接口检测
- 未安装时弹出 `LeanInstallModal`，展示平台相关的 elan 安装命令
- 用户可关闭 Modal 继续使用应用（不强制阻塞，因为用户可能暂时只想用聊天功能）
- Lean4 路径仍由 `LEAN_BIN_PATH` 环境变量配置，默认从 PATH 查找

**install_guide 设计**：返回平台标识（`windows`/`macos`/`linux`/`all`）+ 描述 + 命令 + 链接。前端按 `navigator.userAgent` 过滤展示对应平台步骤。

### 2.3 Release 流程：tag 触发三平台并行构建

**决策**：用 GitHub Actions 的 `release.yml`，tag 推送触发，三平台并行构建并自动上传到 GitHub Release。

**触发条件**：`on.push.tags: ['v*']`

**矩阵**：
| 平台 | runner | target | 产物 |
|---|---|---|---|
| Windows | windows-latest | x86_64-pc-windows-msvc | `.msi` + `.exe`（NSIS） |
| macOS (Apple Silicon) | macos-latest | aarch64-apple-darwin | `.dmg` + `.app` |
| macOS (Intel) | macos-latest | x86_64-apple-darwin | `.dmg` + `.app` |
| Linux | ubuntu-22.04 | x86_64-unknown-linux-gnu | `.deb` + `.AppImage` |

**关键步骤**：
1. `actions/checkout@v5`
2. `dtolnay/rust-toolchain@stable` 安装 Rust（带 `targets` 参数支持交叉编译）
3. `cargo install tauri-cli`（不带 `--version`，避免 semver 解析错误）
4. Linux 安装 webkit2gtk 等系统依赖
5. `actions/setup-node@v5` + `npm install` 构建前端
6. `actions/cache@v5` 缓存 Cargo registry/git/target
7. `tauri-apps/tauri-action@v0` 一站式构建 + 创建 Release + 上传产物

**未签名说明**：开源项目不签名，macOS 用户首次启动需在「系统设置 → 隐私与安全性」中允许运行；Windows 可能显示 SmartScreen 警告，点击「仍要运行」即可。

### 2.4 CI 前端测试补全

**背景**：Phase 1.5 的 `ci.yml` 中 frontend job 只跑 `npm run build`，缺失 `npm test`，导致 49 个前端组件测试（Phase 2 后增至 97 个）未在 CI 中执行。

**修复**：在 `npm run build` 前增加 `npm test -- --run` 步骤。

## 3. 已修复的 Bug

### 3.1 lib.rs 启动 panic（关键风险）

详见 2.1 节。修复前：`expect("DEEPSEEK_API_KEY must be set")`；修复后：`ok()` 容错 + `SharedChatClient` 延迟初始化。

### 3.2 RwLockReadGuard 跨 await 持有

初次实现 `SharedChatClient::chat` 时直接在 guard 上调用 `.await`，编译失败（`RwLockReadGuard` 不是 `Send`）。修复：先 clone 出 `DeepSeekClient`，drop guard，再调用 chat。

## 4. 测试覆盖

Phase 2 新增测试：

**Rust 端**（共 129 个，新增 18 个）：
- `src/deepseek/shared.rs`：8 个（未配置路径、配置路径、clone 共享状态、replace_client 等）
- `src/api/routes.rs`：6 个 settings 路由测试（GET 状态、POST 设置、空 key、缺字段等）
- `src/lean/runner.rs`：4 个 check_version / lean_bin 测试

**前端**（共 97 个，新增 48 个）：
- `useSettings.ts`：11 个（启动拉取、setApiKey 成功/失败、空 key 防御、refresh 等）
- `SettingsModal.tsx`：15 个（条件渲染、表单提交、显示/隐藏 key、forceOpen 等）
- `useLeanInstall.ts`：7 个（启动拉取、installed 状态、fetch 失败回退等）
- `LeanInstallModal.tsx`：15 个（条件渲染、平台过滤、命令复制、关闭按钮等）

## 5. 易被忽视的细节

1. **`tracing_subscriber::fmt().try_init()` 而非 `.init()`**：Tauri 桌面壳中 `main.rs` 已初始化全局 subscriber，`lib.rs::run()` 再次 `init()` 会 panic。`try_init()` 在已初始化时返回 Err，我们忽略即可。
2. **`cargo fmt --all -- --check` 必须带 `--check`**：不带 `--check` 只是格式化，不返回非零退出码，CI 无法捕获格式错误。
3. **批处理脚本必须纯 ASCII**：`run-web.bat` / `run-tauri.bat` 中含中文会导致 Windows cmd 解析错误。
4. **`run-tauri.bat` 用 goto labels**：嵌套 `if` 块带括号在 Windows cmd 中容易解析失败，改用 goto labels 更稳健。
5. **`cargo install tauri-cli` 不带 `--version`**：带 `--version "^2"` 会触发 semver 解析错误。
6. **`src-tauri/Cargo.toml` 的 `custom-protocol` feature**：Tauri 生产构建必需，不能移除。
7. **`src-tauri` rust-version 1.77**：低于 1.77 编译失败。
8. **CI 用 Node.js 22+ + actions/checkout@v5 + actions/setup-node@v5 + actions/cache@v5**：Node.js 20 已弃用，会触发 CI 失败。
9. **Lean4 CI 安装用官方 elan installer**：不用 `lean-action`（在 ubuntu-latest 上 `auto-config: true` 可能失败）。
10. **`.env` 在 `.gitignore` 中**：绝不提交，含真实 API Key。
11. **`tauri.conf.json` 的 `bundle.targets` 默认 `"all"`**：同时产出 .msi + .exe，让用户有选择。
12. **Tauri 2.x 在 Windows 硬编码 `C:\Users\<user>\AppData\Local\tauri` 缓存路径**：`TAURI_BUNDLER_CACHE_DIR` 环境变量无效，沙箱环境需放行该路径或改用 NSIS-only。
13. **Tauri MSI 安装包生成需要 WiX 3.14 工具链**：首次构建会自动下载到上述缓存路径。
14. **`LeanRunner::run` 的 IO 错误分支仍要检测 `sorry`/`admit`**：即使 lean 二进制不存在，含 sorry 的代码也不能逃过安全检查。
15. **`SharedChatClient::chat` 必须先 clone 再 await**：`RwLockReadGuard` 不是 `Send`，不能跨 await 持有。

## 6. 发布流程（v0.2.0 示例）

```bash
# 1. 确认工作区干净
git status

# 2. 同步版本号（tauri.conf.json / Cargo.toml / src-tauri/Cargo.toml / frontend/package.json）
# 已在 P1-7 步骤完成

# 3. 提交所有改动
git add .
git commit -m "phase 2: tauri build, runtime api key ui, lean4 install guide, release workflow"

# 4. 推送到 main
git push origin main

# 5. 打 tag 并推送（触发 release.yml）
git tag v0.2.0
git push origin v0.2.0

# 6. 查看 release workflow 运行状态
gh run list --workflow=release.yml --limit 1

# 7. 三平台构建约 20-40 分钟，完成后在 Releases 页面查看产物
# https://github.com/wytyKen/DeepSeek-LeanSpark/releases
```

## 7. 后续规划（Phase 3 候选）

- **API Key 持久化**：当前重启后需重新输入。Phase 3 可考虑用 Tauri 的 `tauri-plugin-stronghold` 加密存储。
- **Lean4 自动安装**：当前只引导用户手动安装。Phase 3 可在 Tauri 内提供"一键安装 elan"按钮（用 `tauri-plugin-shell` 执行安装命令）。
- **应用签名**：macOS 公证（notarization）+ Windows 代码签名，消除"未知开发者"警告。
- **自动更新**：集成 `tauri-plugin-updater`，发布新版本后用户应用内自动检测并更新。
- **多语言**：当前 UI 仅中文。Phase 3 可加 i18n 支持。
- **多模型支持**：当前仅 DeepSeek。Phase 3 可加 OpenAI / Anthropic 等多模型切换。
