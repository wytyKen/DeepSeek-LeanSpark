# DeepSeek-LeanSpark

> AI 辅助形式化数学工具：DeepSeek 大模型 + Lean4 形式化证明助手。

DeepSeek-LeanSpark 把 DeepSeek 大模型的推理能力与 Lean4 形式化验证器结合在一个对话式工作台中：用户用自然语言描述证明目标，Agent 自动生成 Lean4 代码、调用工具验证、根据编译器反馈迭代修复，最终产出可通过 `lean` 检查的形式化证明。前端提供聊天面板、工作区文件树、证明依赖图、LaTeX 公式渲染，后端通过工具调用循环（tool-calling loop）把 LLM 与 Lean 编译器、文件系统、证明状态检索等能力串联起来。

## 核心特性

- **对话式证明生成**：自然语言输入 → Agent 自动生成并迭代 Lean4 代码
- **工具调用循环**：Agent 自主调用 `run_lean_check` / `read_file` / `write_file` / `search` / `proof_state` 五个工具
- **形式化验证闭环**：每一步 Lean 代码都经 `lean` 编译器实编译，含 `sorry`/`admit` 时自动告警
- **工作区管理**：多文件项目、文件树浏览、代码编辑器
- **证明依赖图**：自动解析 Lean 代码中的 `theorem`/`lemma` 声明与引用关系，可视化依赖结构
- **LaTeX 渲染**：聊天中的数学公式用 KaTeX 渲染，支持点击放大
- **双形态运行**：Web 浏览器形态（开发/部署灵活）+ Tauri 原生桌面形态（单一可执行文件）
- **安全沙箱**：文件工具强制路径校验，禁止 `..` 越界访问工作区外文件

## 两种使用方式

### 方式一：从源码运行（Web 形态，Phase 1 已可用）

适合开发者和愿意自行搭建环境的用户。需要安装 Rust、Node.js、Lean4 三个工具链。

#### 环境要求

| 依赖 | 版本 | 用途 | 安装 |
|---|---|---|---|
| Rust | 1.75+ stable | 编译后端 | https://rustup.rs |
| Node.js | 18+ | 编译前端 | https://nodejs.org |
| Lean4 | 任意稳定版 | 形式化证明验证 | 通过 [elan](https://github.com/leanprover/elan) 安装 |
| DeepSeek API Key | — | 调用 LLM | https://platform.deepseek.com |

#### 步骤（Windows）

```powershell
# 1. 克隆仓库
git clone https://github.com/wytyKen/DeepSeek-LeanSpark.git
cd DeepSeek-LeanSpark

# 2. 配置 API Key
cd DeepSeek-LeanSpark
copy .env.example .env
# 编辑 .env，填入 DEEPSEEK_API_KEY=sk-xxxxxxxx

# 3. 一键启动（回到仓库根）
cd ..
.\run-web.bat
```

`run-web.bat` 会自动启动后端（`cargo run`，监听 3000）和前端（`npm run dev`，监听 5173，自动打开浏览器）。

#### 步骤（Linux / macOS）

```bash
# 1. 克隆
git clone https://github.com/wytyKen/DeepSeek-LeanSpark.git
cd DeepSeek-LeanSpark/DeepSeek-LeanSpark

# 2. 配置 API Key
cp .env.example .env
# 编辑 .env 填入 DEEPSEEK_API_KEY

# 3. 启动后端（终端 1）
cargo run

# 4. 启动前端（终端 2）
cd frontend
npm install
npm run dev -- --open
```

访问 http://localhost:5173 开始使用。

#### 安装 Lean4

Lean4 是形式化验证的必需依赖，未安装时 `run_lean_check` 工具会失败（但应用本身可启动）。

```bash
# Linux / macOS
curl https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.sh -sSf | sh

# Windows PowerShell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.ps1" -OutFile elan-init.ps1
./elan-init.ps1

# 验证
lean --version
```

### 方式二：下载安装包运行（Phase 2 已完成）

Phase 2 已产出可分发的原生桌面安装包，让终端用户**无需安装 Rust/Node.js 工具链**，只下载一个安装包即可使用（仍需安装 Lean4，应用内提供引导）。

#### 终端用户

1. 前往 GitHub 仓库的 **[Releases](https://github.com/wytyKen/DeepSeek-LeanSpark/releases)** 页面
2. 下载对应平台的安装包：
   - Windows: `DeepSeek-LeanSpark_<version>_x64_en-US.msi`（推荐，标准格式）或 `_x64-setup.exe`（NSIS）
   - macOS: `DeepSeek-LeanSpark_<version>_aarch64.dmg`（Apple Silicon M1/M2）/ `_x64.dmg`（Intel）
   - Linux: `DeepSeek-LeanSpark_<version>_amd64.deb` 或 `.AppImage`
3. 安装并启动应用
4. 在应用内「设置」中填入 DeepSeek API Key（从 https://platform.deepseek.com 获取）
5. 应用会自动检测 Lean4 是否已安装；若未安装，按提示用 elan 安装：
   - Windows PowerShell: `Invoke-WebRequest -Uri "https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.ps1" -OutFile elan-init.ps1; ./elan-init.ps1`
   - macOS/Linux: `curl https://raw.githubusercontent.com/leanprover/elan/elan-init/elan-init.sh -sSf | sh`
6. 安装 Lean4 后重启应用，开始使用

> **注**：未配置 API Key 也能进入应用界面（应用启动时自动弹出设置 Modal）；未安装 Lean4 时 Agent 仍可生成证明代码但无法验证正确性。
> macOS 用户首次启动需在「系统设置 → 隐私与安全性」中允许运行（开源项目未签名）。

#### 开发者自行构建安装包

```bash
git clone https://github.com/wytyKen/DeepSeek-LeanSpark.git
cd DeepSeek-LeanSpark/DeepSeek-LeanSpark
cp .env.example .env  # 填入 API Key

# 安装 Tauri CLI
cargo install tauri-cli

# 打包（产物在 src-tauri/target/release/bundle/）
cargo tauri build
```

## 仓库结构

```
deepseek-leanspark/
├── DeepSeek-LeanSpark/      # 项目主代码
│   ├── src/                 # 后端 Rust 源码（agent / api / deepseek / lean / tools / workspace / proof_graph）
│   ├── frontend/            # 前端 React 源码（Vite + TypeScript + KaTeX + React Flow）
│   ├── src-tauri/           # Tauri 桌面壳（Phase 2）
│   ├── tests/               # Rust 集成测试
│   ├── prompts/             # Agent system prompt
│   └── .env.example         # API Key 配置模板
├── docs/                    # 跨阶段设计与实现文档
├── run-web.bat              # Web 形态一键启动（Windows）
├── run-tauri.bat            # Tauri 形态一键启动（Windows）
└── README.md                # 本文件
```

## 开发者文档

| 主题 | 文档 |
|---|---|
| Phase 1 完整实现说明（架构、API、工具、测试） | [DeepSeek-LeanSpark/README.md](./DeepSeek-LeanSpark/README.md) |
| Phase 1 设计文档 | [docs/phase1.md](./docs/phase1.md) |
| Phase 1.5 增强设计（Tauri、右侧栏、依赖图） | [docs/phase1.5-design.md](./docs/phase1.5-design.md) |
| Phase 2 设计文档（原生分发、运行时配置、release 流程） | [docs/phase2.md](./docs/phase2.md) |
| 项目知识基线 | [docs/leanspark-guide.html](./docs/leanspark-guide.html) |

## 测试

```bash
cd DeepSeek-LeanSpark

# Rust 单元测试（129 个，无需后端运行）
cargo test --lib

# 前端组件测试（97 个）
cd frontend && npm test -- --run

# Rust 集成烟雾测试（需先启动后端：cargo run）
cargo test --test api_smoke
```

## 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust + Axum + Tokio + Reqwest + Serde |
| 前端 | React + TypeScript + Vite + KaTeX + React Flow |
| 桌面壳 | Tauri 2.x |
| LLM | DeepSeek Chat / Reasoner（支持 thinking 模式） |
| 形式化 | Lean4 + elan |
| 测试 | cargo test + Vitest + @testing-library/react |

## License

见 [DeepSeek-LeanSpark/LICENSE](./DeepSeek-LeanSpark/LICENSE)。
