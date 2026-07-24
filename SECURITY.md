# 安全策略

## 报告漏洞

如果你发现安全漏洞，请**不要**通过公开 Issue 报告。

请发送邮件至：security@example.com（替换为实际邮箱）

邮件内容应包含：
- 漏洞描述
- 复现步骤
- 影响范围
- 建议修复方案（如有）

我们会在 72 小时内确认收到，并在 7 天内给出初步评估。

## 支持版本

| 版本 | 支持状态 |
|---|---|
| Phase 1 (0.x) | 安全更新 |
| Phase 2 (未发布) | — |

## 已知安全考量

### 1. Lean4 代码执行

DeepSeek-LeanSpark 后端会调用 `lean` 可执行文件编译 AI 生成的代码。这本身是设计意图，但需注意：

- **临时文件**：编译使用系统临时目录（`std::env::temp_dir`），文件名含 UUID，编译后立即删除
- **无沙箱**：Phase 1 不对 `lean` 进程做沙箱隔离。Lean4 编译器本身不会执行被编译的代码（仅类型检查），但若部署在多用户环境，建议用容器隔离
- **Phase 2 改进**：将引入进程级沙箱（如 firejail / Windows AppContainer）

### 2. sorry / admit 检测

为防止 AI 用 `sorry` 跳过证明（"P=NP 式造假"），系统在两处检测：
- `LeanRunner::run` 检查源码是否含 `sorry` 或 `admit`
- `LeanCheckTool::call` 在返回给 AI 的结果中加 WARNING

但这只是软性提示，不阻止编译。Phase 2 将增加可选的硬性拒绝策略。

### 3. API Key 保护

- `DEEPSEEK_API_KEY` 通过 `.env` 加载，`.env` 已在 `.gitignore` 中
- 后端日志不会打印 API Key
- 前端不接触 API Key，所有 LLM 调用由后端代理

### 4. CORS

Phase 1 开发环境使用 `CorsLayer::permissive()` 允许所有来源。生产部署若前后端同源，应移除该层；若跨源，应配置精确的 `Access-Control-Allow-Origin`。

### 5. 输入验证

- `/api/chat` 与 `/api/lean/check` 通过 serde 反序列化做基本类型校验
- 未对消息长度做硬性限制，依赖 DeepSeek API 自身的 token 上限。Phase 2 将增加后端侧长度限制

## 依赖安全

- 依赖更新通过 `cargo update` / `npm update`，由维护者定期执行
- 引入新依赖需在 PR 中说明并经过评审
- 建议启用 Dependabot（见 `.github/dependabot.yml`）

## 报告流程时间线

1. **Day 0**：收到报告，确认收到
2. **Day 1-7**：评估漏洞严重程度
3. **Day 7-30**：开发修复补丁
4. **Day 30**：发布修复版本，公开致谢（如报告者同意）
