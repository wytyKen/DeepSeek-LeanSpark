# Phase 1.5 设计文档：UI 升级与工作区能力

本文档描述 Phase 1.5 的设计与实现，是 Phase 1（Web MVP）与 Phase 2（Tauri 原生壳）之间的过渡阶段。所有改动仍在 Web 形态下完成，Phase 2 时用 Tauri 包壳即可。

> **后续阶段**：Phase 2 已完成原生分发、运行时 API Key 配置、Lean4 安装引导与 GitHub Releases 自动化，详见 [phase2.md](./phase2.md)。

---

## 0. 目标与范围

### 目标
1. 聊天面板按 Trae Work 风格重做（用户气泡靠右、助手消息折叠展开、任务耗时显示）
2. 右侧栏改为三 Tab：资源管理器 / 证明依赖图 / 公式
3. 支持打开工作区文件夹，前端展示文件树
4. Agent 新增 `read_file`/`write_file` 工具，可读写工作区内文件
5. LaTeX 公式渲染（KaTeX），支持复制源码、点击放大
6. 证明依赖图（静态解析版），用 React Flow 渲染

### 非目标（Phase 2/3 处理）
- Tauri 原生壳（Phase 2）
- 系统文件对话框、原生菜单、托盘（Phase 2）
- Lean4 LSP 精确依赖分析（Phase 3）
- Git 集成（暂不考虑）
- Mathlib 在线搜索（Phase 3）

---

## 1. 术语规范

| 术语 | 标准用法 |
|---|---|
| 工作区（Workspace） | Agent 当前打开的项目文件夹 |
| 资源管理器（Explorer） | 右侧栏"资源管理器"Tab 的标准名称，用于浏览工作区文件树 |
| 证明依赖图 | 节点=theorem/lemma 声明，边=A 的证明调用了 B |
| 公式 / 定理 / 引理 / 命题 | 严格区分；Lean4 中统称 declaration |
| 对话上下文 | 一次会话的完整消息历史 |
| 智能体（Agent） | 中文语境规范用法 |

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────────┐
│  顶部栏：DeepSeek-LeanSpark | thinking 切换 | 工作区路径   │
├──────────────────────────┬──────────────────────────────┤
│  左侧：聊天面板           │  右侧栏（三 Tab 切换）        │
│  ┌──────────────────┐   │  [资源管理器][证明依赖图][公式]│
│  │ 用户气泡（靠右）  │   │ ┌──────────────────────────┐ │
│  └──────────────────┘   │ │ 文件树 / 依赖图 / 公式列表│ │
│  LeanSpark 任务耗时 Xm  │ │                          │ │
│  ▶ 思考（折叠）         │ │                          │ │
│  ▶ 调用工具（折叠）     │ │                          │ │
│  ▶ 工具结果（折叠）     │ │                          │ │
│  ─────────────────      │ │                          │ │
│  回答正文（Markdown）   │ │                          │ │
│  N 个文件已更改         │ │                          │ │
│  ─────────────────      │ │                          │ │
│  [输入框]               │ │                          │ │
└──────────────────────────┴──────────────────────────────┘
```

---

## 3. 后端改动

### 3.1 新增模块：`src/workspace/`

```
src/workspace/
├── mod.rs          # 模块导出
├── manager.rs      # WorkspaceManager：跟踪当前工作区路径
└── paths.rs        # 路径安全校验（防穿越）
```

**`WorkspaceManager`**：
- 持有当前工作区根路径 `Option<PathBuf>`（`Arc<RwLock<Option<PathBuf>>>`）
- `open(path)` / `current()` / `close()`
- `list_tree()`：返回递归文件树 JSON（深度限制 5 层，排除 `.` 开头目录、`target/`、`node_modules/`、`.lake/`）
- `read_file(rel_path)` / `write_file(rel_path, content)`：路径必须在根内

**路径安全**：
```rust
fn ensure_within(root: &Path, target: &Path) -> Result<PathBuf> {
    let canonical_root = root.canonicalize()?;
    let canonical_target = target.canonicalize()?;
    canonical_target
        .strip_prefix(&canonical_root)
        .map_err(|_| anyhow!("path escapes workspace"))?;
    Ok(canonical_target)
}
```

写入前用 `canonicalize` 防符号链接穿越。

### 3.2 新增 API 路由

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/workspace/open` | body: `{"path": "..."}`，设置工作区 |
| GET | `/api/workspace/current` | 返回当前工作区路径与文件树 |
| POST | `/api/workspace/close` | 关闭工作区 |
| GET | `/api/workspace/tree` | 重新拉取文件树 |
| POST | `/api/workspace/read` | body: `{"path": "rel/path.lean"}` |
| POST | `/api/workspace/write` | body: `{"path": "...", "content": "..."}` |
| POST | `/api/proof-graph` | body: `{"code": "..."}`，返回 `{nodes, edges}` |

### 3.3 新增工具：`read_file` / `write_file`

注册到 `ToolRegistry`，仅当工作区已打开时可用。

```rust
// read_file: 读取工作区内相对路径文件
// write_file: 写入工作区内相对路径文件（创建或覆盖）
```

工具规格（OpenAI function schema）：
```json
{
  "name": "read_file",
  "parameters": {"path": {"type": "string", "description": "工作区内相对路径"}}
}
{
  "name": "write_file",
  "parameters": {
    "path": {"type": "string"},
    "content": {"type": "string"}
  }
}
```

### 3.4 证明依赖图解析

新增 `src/proof_graph/parser.rs`：
- 用正则提取 `theorem`/`lemma` 声明：`^(theorem|lemma)\s+(\w+)`
- 用正则提取 tactic 调用中的引理名：`apply\s+(\w+)`、`exact\s+(\w+)`、`rw\s+\[(.+?)\]`、`simp\s+\[(.+?)\]`、`have\s+\w+\s*:=\s*(\w+)`
- 节点：声明的定理/引理；外部引理（mathlib/标准库）标记为 `external: true`
- 边：从声明节点指向其证明中调用的引理节点
- 返回 `{nodes: [{id, name, kind, external}], edges: [{from, to}]}`

### 3.5 `AgentEvent` 扩展

新增字段以支持前端展示：

```rust
pub struct AgentEvent {
    pub kind: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<Value>,
    // 新增
    pub files_changed: Vec<String>,   // 被 write_file 修改的相对路径
    pub files_created: Vec<String>,   // 被 write_file 创建的新文件
    pub duration_ms: Option<u64>,     // 仅 answer 事件填，整轮耗时
}
```

`AgentLoop::run` 在结束时计算总耗时填入 answer 事件；`write_file` 工具调用结果中带 `files_changed`/`files_created`。

---

## 4. 前端改动

### 4.1 新增依赖

```json
{
  "dependencies": {
    "@uiw/react-codemirror": "^4.23.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-markdown": "^9.0.1",
    "remark-gfm": "^4.0.0",
    "rehype-katex": "^7.0.0",          // 新增：LaTeX 渲染
    "katex": "^0.16.0",                // 新增
    "reactflow": "^11.11.0",           // 新增：证明依赖图渲染
    "dagre": "^0.8.5"                  // 新增：图自动布局
  }
}
```

> 文件树使用原生递归实现（见 `FileTree.tsx`），不依赖 react-arborist。

### 4.2 组件结构

```
frontend/src/
├── App.tsx                          # 顶层布局
├── components/
│   ├── chat/
│   │   ├── ChatPanel.tsx            # 左侧整体
│   │   ├── MessageBubble.tsx        # 单条消息（用户/助手分支）
│   │   ├── UserBubble.tsx           # 用户气泡：靠右灰色圆角
│   │   ├── AssistantMessage.tsx     # 助手消息：折叠展开 + 耗时
│   │   ├── EventCollapse.tsx        # 思考/工具调用/工具结果折叠组件
│   │   └── ChatInput.tsx            # 输入框
│   ├── sidebar/
│   │   ├── RightSidebar.tsx         # 右侧栏容器 + Tab 切换
│   │   ├── ExplorerTab.tsx          # 资源管理器 Tab
│   │   ├── FileTree.tsx             # 文件树（react-arborist）
│   │   ├── ProofGraphTab.tsx        # 证明依赖图 Tab
│   │   ├── ProofGraphView.tsx       # React Flow 渲染
│   │   ├── FormulaTab.tsx           # 公式 Tab
│   │   └── FormulaCard.tsx          # 单个公式卡片（含复制/放大）
│   ├── workspace/
│   │   ├── WorkspaceSwitcher.tsx    # 顶部工作区路径显示 + 打开按钮
│   │   └── CodeEditor.tsx           # 改造：点击文件树后在此编辑
│   └── common/
│       ├── LatexModal.tsx           # 公式放大 modal
│       └── CopyButton.tsx           # 复制按钮
├── hooks/
│   ├── useAgent.ts                  # 改造：处理新事件字段、统计文件变更
│   ├── useWorkspace.ts              # 新增：工作区状态管理
│   └── useProofGraph.ts             # 新增：依赖图数据获取
└── styles/
    └── chat.css                     # 聊天面板样式
```

### 4.3 聊天面板样式规范（Trae Work 风格）

**用户气泡**：
- 容器 `display: flex; justify-content: flex-end;`
- 气泡 `max-width: 70%; background: #f0f0f0; border-radius: 12px; padding: 8px 12px;`
- 不显示"你"
- 纯文本，不解析 Markdown

**助手消息**：
- 容器 `display: flex; flex-direction: column; align-items: flex-start;`
- 顶部行：`LeanSpark 任务耗时 Xm XXs`（灰色 12px）
- 折叠区（默认折叠，三角形 `▶`/`▼` 切换）：
  - `思考` —— `reasoning_content`
  - `调用工具: <name>` —— 每个 tool_call 独立折叠
  - `工具结果` —— 每个 tool_result 独立折叠
- 分隔线：`<hr style="border: none; border-top: 1px solid #eee; margin: 8px 0;">`
- 回答正文：`react-markdown` + `remark-gfm` + `rehype-katex`
- 文件变更标记：`N 个文件已更改 · N 个文件已生成`（灰色 12px，无则不显示）
- 不显示"LeanSpark"标签字（与耗时行合并）

### 4.4 LaTeX 渲染

- `react-markdown` 配置 `rehype-katex`
- 引入 `katex/dist/katex.min.css`
- 块级公式 `$$...$$` 自动渲染为居中显示
- 每个块级公式右上角浮一个复制按钮（`CopyButton`），复制原始 LaTeX 源码
- 点击公式块弹出 `LatexModal`，大字号显示，含关闭按钮和复制按钮
- 行内公式 `$...$` 仅渲染，不加按钮

### 4.5 资源管理器 Tab

- 顶部：当前工作区路径 + "打开文件夹"按钮 + "关闭"按钮
- "打开文件夹"：Phase 1.5 Web 形态用 prompt 输入路径；Phase 2 Tauri 原生壳下用 `@tauri-apps/plugin-dialog` 系统文件对话框
- 文件树用原生递归实现（不依赖 react-arborist，避免虚拟化对小工作区过度复杂）
- 点击 `.lean` 文件 → 在底部 `CodeEditor` 打开（可编辑）
- 点击 `.md`/`.txt` 文件 → 同上
- 排除规则：`target/`、`node_modules/`、`.lake/`、`.git/`、`.` 开头文件
- 文件被 `write_file` 修改后自动刷新树

### 4.6 证明依赖图 Tab

- 取最近一次 `run_lean_check` 提交的代码，调 `/api/proof-graph` 获取图数据
- React Flow 渲染：
  - 节点：`theorem`（蓝色）、`lemma`（绿色）、外部引理（灰色虚线边框）
  - 边：有向，从声明节点指向依赖节点
  - 支持缩放、拖拽、自动布局（dagre）
- 无代码时显示"尚无证明依赖图"

### 4.7 公式 Tab

- 收集当前会话所有助手回答中出现的块级 LaTeX 公式
- 每个公式卡片：
  - 上方：KaTeX 渲染结果
  - 下方：来源消息缩略（"来自第 N 轮对话"）
  - 右上角：复制 LaTeX 源码按钮
  - 点击：放大 modal
- 无公式时显示"尚无公式"

---

## 5. 数据流

### 5.1 打开工作区

```
用户点击"打开文件夹" → 输入路径 → POST /api/workspace/open
  → 后端 canonicalize、记录路径、返回文件树
  → 前端 useWorkspace 缓存路径+树 → 资源管理器渲染
```

### 5.2 Agent 写文件

```
LLM 决定调用 write_file → tool_args: {path, content}
  → WorkspaceManager.write_file 校验路径、写入
  → 返回 {path, created: bool, bytes: N}
  → AgentEvent 带 files_changed/files_created
  → 前端 useAgent 累加 → 助手消息底部显示"1 个文件已更改"
  → 自动刷新文件树
```

### 5.3 证明依赖图

```
助手消息中 run_lean_check 的 tool_args.lean_code
  → 前端取最近一次 → POST /api/proof-graph {code}
  → 后端静态解析 → {nodes, edges}
  → React Flow 渲染
```

---

## 6. 测试计划

### 6.1 后端测试（`tests/`）

新增 `tests/workspace_smoke.rs`：
- `workspace_open_and_list_tree` —— 打开临时目录，验证文件树结构
- `workspace_read_file_within_root` —— 读工作区内文件成功
- `workspace_read_file_escape_root` —— 路径穿越被拒绝
- `workspace_write_file_creates_new` —— 写新文件成功
- `workspace_write_file_escape_root` —— 写路径穿越被拒绝
- `proof_graph_parses_simple_theorem` —— 解析 `theorem t : ... := by apply foo` 返回正确节点边

扩展 `tests/api_smoke.rs`：
- `tools_list_includes_read_write_file` —— 工具列表包含 read_file/write_file（仅工作区打开后）
- `chat_with_file_write_marks_files_changed` —— 完整对话中 write_file 调用导致前端可见的 files_changed

### 6.2 前端测试

- 手动验证：浏览器打开，加载工作区，文件树显示
- 手动验证：聊天面板用户气泡右对齐、助手消息折叠展开
- 手动验证：LaTeX 公式渲染、复制、放大
- 手动验证：依赖图渲染、缩放拖拽

### 6.3 回归测试

- 原 5 个 `api_smoke.rs` 测试全部保持通过
- `cargo clippy --all-targets -- -D warnings` 通过
- `cargo fmt --check` 通过
- `npm run build` 通过

---

## 7. 风险与缓解

| 风险 | 缓解 |
|---|---|
| Agent 写文件覆盖用户重要文件 | 工作区限制 + 路径校验 + 日志记录所有写操作 |
| 静态解析依赖图漏报 | 标注"静态解析，可能不完整"，Phase 3 接 LSP |
| `react-arborist` 在大目录性能差 | 深度限制 5 层 + 文件数限制 1000 |
| KaTeX 不支持的 LaTeX 宏 | 限制 LLM 输出标准 LaTeX，禁用 `\def` 等宏 |
| Phase 1 已通过测试被破坏 | 所有改动新增模块，不修改现有 API 行为 |

---

## 8. 文件改动清单

### 新增
- `DeepSeek-LeanSpark/src/workspace/mod.rs`
- `DeepSeek-LeanSpark/src/workspace/manager.rs`
- `DeepSeek-LeanSpark/src/workspace/paths.rs`
- `DeepSeek-LeanSpark/src/proof_graph/mod.rs`
- `DeepSeek-LeanSpark/src/proof_graph/parser.rs`
- `DeepSeek-LeanSpark/src/tools/read_file.rs`
- `DeepSeek-LeanSpark/src/tools/write_file.rs`
- `DeepSeek-LeanSpark/tests/workspace_smoke.rs`
- 前端 `components/chat/*` 6 个文件
- 前端 `components/sidebar/*` 6 个文件
- 前端 `components/workspace/*` 2 个文件
- 前端 `components/common/*` 2 个文件
- 前端 `hooks/useWorkspace.ts`、`useProofGraph.ts`
- 前端 `styles/chat.css`

### 修改
- `DeepSeek-LeanSpark/src/lib.rs` —— 加 workspace/proof_graph 模块、AppState 加 WorkspaceManager
- `DeepSeek-LeanSpark/src/api/routes.rs` —— 加 workspace/proof-graph 路由
- `DeepSeek-LeanSpark/src/tools/mod.rs` —— 注册 read_file/write_file
- `DeepSeek-LeanSpark/src/agent/agent_loop.rs` —— AgentEvent 加 files_changed/files_created/duration_ms，run 末尾计算耗时
- `DeepSeek-LeanSpark/frontend/package.json` —— 加依赖
- `DeepSeek-LeanSpark/frontend/src/App.tsx` —— 改为左右两栏布局 + 顶部工作区切换
- `DeepSeek-LeanSpark/frontend/src/types.ts` —— AgentEvent 加字段
- `DeepSeek-LeanSpark/frontend/src/hooks/useAgent.ts` —— 处理新字段
- `DeepSeek-LeanSpark/frontend/src/components/CodeEditor.tsx` —— 适配工作区文件编辑
- `DeepSeek-LeanSpark/prompts/agent-prompt.md` —— 加 read_file/write_file 使用说明

### 删除
- `DeepSeek-LeanSpark/frontend/src/components/ChatPanel.tsx` —— 拆分为 `chat/` 子目录
- `DeepSeek-LeanSpark/frontend/src/components/ProofState.tsx` —— 功能并入证明依赖图 Tab

---

## 9. 验收标准

| 编号 | 验收点 | 标准 |
|---|---|---|
| A1 | 聊天面板用户气泡 | 靠右、灰色圆角、最大宽度 70%、无"你"字 |
| A2 | 聊天面板助手消息 | 显示"LeanSpark 任务耗时 Xm XXs"，思考/工具折叠可展开 |
| A3 | 助手消息文件变更标记 | write_file 后显示"N 个文件已更改" |
| A4 | 右侧栏三 Tab | 资源管理器/证明依赖图/公式可切换 |
| A5 | 资源管理器 | 打开文件夹后展示文件树，点击 .lean 文件在编辑器打开 |
| A6 | LaTeX 渲染 | 块级公式渲染为数学排版，可复制源码、点击放大 |
| A7 | 证明依赖图 | 提交 Lean 代码后渲染节点+边，可缩放拖拽 |
| A8 | 工作区路径安全 | 路径穿越攻击被拒绝（测试覆盖） |
| A9 | 回归测试 | 原 5 个 api_smoke 测试全过 |
| A10 | 构建检查 | clippy + fmt + npm build 全过 |
