# DeepSeek-LeanSpark

AI 辅助形式化数学工具：DeepSeek V4 大模型 + Lean4 形式化证明助手。

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

## 测试

```bash
# 先启动后端：cargo run
# 再跑端到端烟雾测试：
cargo test --test api_smoke
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

## 原生桌面壳（Tauri 2.x）

本项目支持打包为原生桌面应用，无需浏览器，提供系统文件对话框、原生窗口体验。

### 环境要求

- Rust 1.77+
- WebView2（Windows 10+ 自带；macOS 用 WebKit；Linux 用 WebKitGTK）
- Tauri CLI：`cargo install tauri-cli --version "^2"`

### 开发模式（带热重载）

```bash
# 在 DeepSeek-LeanSpark/ 目录下执行
cargo tauri dev
```

这会同时启动：前端 vite dev server（5173）+ Rust 后端（3000）+ Tauri 窗口。

### 生产打包

```bash
# 生成图标（仅首次需要，源图建议 1024x1024 PNG）
cargo tauri icon path/to/your-icon.png

# 打包（Windows 产出 .msi/.exe；macOS 产出 .dmg/.app；Linux 产出 .deb/.AppImage）
cargo tauri build
```

产物在 `src-tauri/target/release/bundle/`。

### Web 形态 vs 原生形态

| 特性 | Web 形态（npm run dev + cargo run） | 原生形态（cargo tauri dev） |
|------|------------------------------------|---------------------------| 
| 文件对话框 | `window.prompt` 输入路径 | 系统原生文件对话框 |
| 窗口 | 浏览器标签页 | 原生窗口 |
| 后端 | 独立进程（cargo run） | Tauri 主进程内 |
| 部署 | 需要浏览器 | 单一可执行文件 |

两种形态共享同一套前后端代码，仅运行环境不同。

## 项目结构

见 [`../docs/phase1.md`](../docs/phase1.md)。
