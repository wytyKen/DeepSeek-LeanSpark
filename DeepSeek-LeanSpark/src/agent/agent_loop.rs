use crate::deepseek::ChatClient;
use crate::deepseek::ChatResponse;
use crate::tools::ToolRegistry;
use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

const MAX_ITERATIONS: usize = 20;

pub struct AgentLoop {
    client: Arc<dyn ChatClient>,
    tools: Arc<ToolRegistry>,
}

impl AgentLoop {
    pub fn new(client: Arc<dyn ChatClient>, tools: Arc<ToolRegistry>) -> Self {
        Self { client, tools }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentEvent {
    pub kind: String, // "thinking" | "tool_call" | "tool_result" | "answer" | "error"
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<Value>,
    /// 被 write_file 修改的相对路径（仅 tool_result 事件填）
    #[serde(default)]
    pub files_changed: Vec<String>,
    /// 被 write_file 创建的新文件相对路径（仅 tool_result 事件填）
    #[serde(default)]
    pub files_created: Vec<String>,
    /// 整轮 Agent 循环耗时（毫秒），仅 answer/error 事件填
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl AgentEvent {
    fn plain(kind: &str, content: String) -> Self {
        Self {
            kind: kind.to_string(),
            content,
            tool_name: None,
            tool_args: None,
            files_changed: Vec::new(),
            files_created: Vec::new(),
            duration_ms: None,
        }
    }
}

impl AgentLoop {
    /// 运行 Agent 循环。
    ///
    /// - `user_message`: 用户本轮输入
    /// - `history`: 之前的完整消息历史（不含 system prompt）
    /// - `use_thinking`: 是否启用 thinking 模式
    ///
    /// 返回 (本轮产生的事件列表, 完整的新消息历史含 system prompt)
    pub async fn run(
        &self,
        user_message: &str,
        history: &[Value],
        use_thinking: bool,
    ) -> Result<(Vec<AgentEvent>, Vec<Value>)> {
        let started = Instant::now();
        let system_prompt = crate::agent::prompt::load_system_prompt()?;
        let tool_specs = self.tools.specs();

        let mut messages: Vec<Value> = Vec::with_capacity(history.len() + 2);
        messages.push(json!({ "role": "system", "content": system_prompt }));
        for h in history {
            messages.push(h.clone());
        }
        messages.push(json!({ "role": "user", "content": user_message }));

        let mut events = Vec::new();
        // 整轮累计的文件变更（供最终 answer 事件回带，方便前端展示汇总）
        let mut all_files_changed: Vec<String> = Vec::new();
        let mut all_files_created: Vec<String> = Vec::new();

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
                events.push(AgentEvent::plain("thinking", rc.clone()));
            }

            match resp.finish_reason.as_str() {
                "stop" => {
                    let answer = resp.content.clone().unwrap_or_default();
                    let duration_ms = started.elapsed().as_millis() as u64;
                    let mut ev = AgentEvent::plain("answer", answer);
                    ev.duration_ms = Some(duration_ms);
                    ev.files_changed = all_files_changed.clone();
                    ev.files_created = all_files_created.clone();
                    events.push(ev);
                    messages.push(resp.to_assistant_message());
                    return Ok((events, messages));
                }
                "tool_calls" => {
                    // thinking 模式下必须把 reasoning_content 一起回传，否则 400
                    messages.push(resp.to_assistant_message());

                    for tc in &resp.tool_calls {
                        let mut call_ev =
                            AgentEvent::plain("tool_call", format!("调用工具: {}", tc.name));
                        call_ev.tool_name = Some(tc.name.clone());
                        call_ev.tool_args =
                            Some(serde_json::from_str(&tc.arguments).unwrap_or(Value::Null));
                        events.push(call_ev);

                        let args: Value =
                            serde_json::from_str(&tc.arguments).unwrap_or(Value::Null);
                        let result = match self.tools.dispatch(&tc.name, &args).await {
                            Ok(output) => output,
                            Err(e) => {
                                let err_msg = format!("工具执行错误: {}", e);
                                let mut err_ev = AgentEvent::plain("error", err_msg.clone());
                                err_ev.tool_name = Some(tc.name.clone());
                                events.push(err_ev);
                                json!({ "error": err_msg }).to_string()
                            }
                        };

                        // 从 write_file 结果中提取文件变更元字段
                        let mut files_changed: Vec<String> = Vec::new();
                        let mut files_created: Vec<String> = Vec::new();
                        if tc.name == "write_file" {
                            if let Ok(parsed) = serde_json::from_str::<Value>(&result) {
                                if let Some(arr) =
                                    parsed.get("__files_changed").and_then(|v| v.as_array())
                                {
                                    files_changed = arr
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect();
                                }
                                if let Some(arr) =
                                    parsed.get("__files_created").and_then(|v| v.as_array())
                                {
                                    files_created = arr
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect();
                                }
                            }
                        }
                        all_files_changed.extend(files_changed.iter().cloned());
                        all_files_created.extend(files_created.iter().cloned());

                        let mut result_ev = AgentEvent::plain("tool_result", result.clone());
                        result_ev.tool_name = Some(tc.name.clone());
                        result_ev.files_changed = files_changed;
                        result_ev.files_created = files_created;
                        events.push(result_ev);

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tc.id,
                            "content": result,
                        }));
                    }
                }
                other => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    let mut ev =
                        AgentEvent::plain("error", format!("意外的 finish_reason: {}", other));
                    ev.duration_ms = Some(duration_ms);
                    events.push(ev);
                    return Ok((events, messages));
                }
            }
        }

        let duration_ms = started.elapsed().as_millis() as u64;
        let mut ev = AgentEvent::plain(
            "error",
            format!("达到最大迭代次数 {}，Agent 循环终止", MAX_ITERATIONS),
        );
        ev.duration_ms = Some(duration_ms);
        events.push(ev);
        Ok((events, messages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deepseek::{ChatResponse, ToolCall};
    use crate::lean::LeanRunner;
    use crate::workspace::WorkspaceManager;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// Mock ChatClient：按预设顺序返回 ChatResponse 队列。
    /// 队列耗尽时 panic，便于发现"未预期额外调用"。
    struct MockChatClient {
        responses: Mutex<VecDeque<ChatResponse>>,
    }

    impl MockChatClient {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
            }
        }
    }

    #[async_trait]
    impl ChatClient for MockChatClient {
        async fn chat(
            &self,
            _messages: &[Value],
            _tools: Option<&[Value]>,
            _tool_choice: Option<&str>,
        ) -> Result<ChatResponse> {
            let mut guard = self.responses.lock().unwrap();
            guard.pop_front().ok_or_else(|| {
                anyhow::anyhow!("MockChatClient: 响应队列已耗尽（未预期的额外调用）")
            })
        }

        async fn chat_with_thinking(
            &self,
            _messages: &[Value],
            _tools: &[Value],
            _reasoning_effort: &str,
        ) -> Result<ChatResponse> {
            let mut guard = self.responses.lock().unwrap();
            guard.pop_front().ok_or_else(|| {
                anyhow::anyhow!("MockChatClient: 响应队列已耗尽（未预期的额外调用）")
            })
        }
    }

    /// 构造 stop 响应（直接回答）
    fn stop_response(content: &str) -> ChatResponse {
        ChatResponse {
            finish_reason: "stop".to_string(),
            content: Some(content.to_string()),
            reasoning_content: None,
            tool_calls: vec![],
        }
    }

    /// 构造 tool_calls 响应
    fn tool_call_response(id: &str, name: &str, args: Value) -> ChatResponse {
        ChatResponse {
            finish_reason: "tool_calls".to_string(),
            content: None,
            reasoning_content: None,
            tool_calls: vec![ToolCall {
                id: id.to_string(),
                name: name.to_string(),
                arguments: serde_json::to_string(&args).unwrap(),
            }],
        }
    }

    /// 构造意外 finish_reason 响应
    fn unexpected_response(reason: &str) -> ChatResponse {
        ChatResponse {
            finish_reason: reason.to_string(),
            content: None,
            reasoning_content: None,
            tool_calls: vec![],
        }
    }

    /// 构造测试用 AgentLoop（注入 MockChatClient + 真实 ToolRegistry，workspace 打开到临时目录）
    async fn make_agent(
        responses: Vec<ChatResponse>,
    ) -> (AgentLoop, tempfile::TempDir, Arc<WorkspaceManager>) {
        // 临时工作区
        let tmp = tempfile::tempdir().unwrap();
        let workspace = Arc::new(WorkspaceManager::new());
        workspace.open(tmp.path().to_str().unwrap()).await.unwrap();

        // LeanRunner 用假路径（测试不触发 lean_check）
        let lean = Arc::new(LeanRunner::new("lean".to_string()));
        let tools = Arc::new(crate::tools::ToolRegistry::new_with_workspace(
            lean,
            workspace.clone(),
        ));

        let client: Arc<dyn ChatClient> = Arc::new(MockChatClient::new(responses));
        let agent = AgentLoop::new(client, tools);
        (agent, tmp, workspace)
    }

    #[tokio::test]
    async fn test_direct_answer_without_tool_calls() {
        // Mock 直接返回 stop，无工具调用
        let (agent, _tmp, _ws) = make_agent(vec![stop_response("证明完成")]).await;

        let (events, messages) = agent.run("证明 1+1=2", &[], false).await.unwrap();

        // 应只有 1 个 answer 事件
        assert_eq!(events.len(), 1, "应有 1 个 answer 事件");
        assert_eq!(events[0].kind, "answer");
        assert_eq!(events[0].content, "证明完成");
        assert!(
            events[0].duration_ms.is_some(),
            "answer 事件应带 duration_ms"
        );

        // messages 应含 system + user + assistant
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[2]["content"], "证明完成");
    }

    #[tokio::test]
    async fn test_tool_call_then_answer() {
        // 第一次：tool_calls 调用 read_file
        // 第二次：stop 返回最终答案
        let (agent, _tmp, _ws) = make_agent(vec![
            tool_call_response("call_1", "read_file", json!({ "path": "nonexistent.lean" })),
            stop_response("已读取文件"),
        ])
        .await;

        let (events, _messages) = agent.run("读文件", &[], false).await.unwrap();

        // 期望事件顺序：tool_call, tool_result, answer
        assert_eq!(events.len(), 3, "应有 tool_call + tool_result + answer");
        assert_eq!(events[0].kind, "tool_call");
        assert_eq!(events[0].tool_name.as_deref(), Some("read_file"));
        assert_eq!(events[1].kind, "tool_result");
        assert_eq!(events[1].tool_name.as_deref(), Some("read_file"));
        assert_eq!(events[2].kind, "answer");
        assert_eq!(events[2].content, "已读取文件");
    }

    #[tokio::test]
    async fn test_tool_error_recovery_continues_loop() {
        // 调用一个不存在的工具 → 工具错误 → 但循环继续 → 第二次 stop
        let (agent, _tmp, _ws) = make_agent(vec![
            tool_call_response("call_1", "nonexistent_tool", json!({})),
            stop_response("错误后继续"),
        ])
        .await;

        let (events, _messages) = agent.run("触发错误", &[], false).await.unwrap();

        // 期望：tool_call, error(工具错误), tool_result(带 error 字符串), answer
        assert_eq!(
            events.len(),
            4,
            "应有 tool_call + error + tool_result + answer"
        );
        assert_eq!(events[0].kind, "tool_call");
        assert_eq!(events[1].kind, "error");
        assert!(
            events[1].content.contains("工具执行错误"),
            "error 事件应含错误信息"
        );
        assert_eq!(events[2].kind, "tool_result");
        assert_eq!(events[3].kind, "answer");
        assert_eq!(events[3].content, "错误后继续");
    }

    #[tokio::test]
    async fn test_files_changed_accumulation_from_write_file() {
        // 调用 write_file 写入 foo.lean，然后 stop
        let (agent, _tmp, _ws) = make_agent(vec![
            tool_call_response(
                "call_1",
                "write_file",
                json!({ "path": "foo.lean", "content": "theorem t : True := by trivial" }),
            ),
            stop_response("已写入文件"),
        ])
        .await;

        let (events, _messages) = agent.run("写文件", &[], false).await.unwrap();

        // 找到 tool_result 事件，验证 files_changed
        let tool_result = events
            .iter()
            .find(|e| e.kind == "tool_result")
            .expect("应有 tool_result 事件");
        assert_eq!(tool_result.tool_name.as_deref(), Some("write_file"));
        assert_eq!(tool_result.files_changed, vec!["foo.lean".to_string()]);
        assert_eq!(tool_result.files_created, vec!["foo.lean".to_string()]);

        // answer 事件应汇总累计 files_changed
        let answer = events
            .iter()
            .find(|e| e.kind == "answer")
            .expect("应有 answer 事件");
        assert_eq!(answer.files_changed, vec!["foo.lean".to_string()]);
        assert_eq!(answer.files_created, vec!["foo.lean".to_string()]);
    }

    #[tokio::test]
    async fn test_multiple_write_files_accumulate_all() {
        // 两次 write_file，然后 stop
        let (agent, _tmp, _ws) = make_agent(vec![
            tool_call_response(
                "call_1",
                "write_file",
                json!({ "path": "a.lean", "content": "x" }),
            ),
            tool_call_response(
                "call_2",
                "write_file",
                json!({ "path": "b.lean", "content": "y" }),
            ),
            stop_response("两文件已写"),
        ])
        .await;

        let (events, _messages) = agent.run("写两文件", &[], false).await.unwrap();

        let answer = events
            .iter()
            .find(|e| e.kind == "answer")
            .expect("应有 answer 事件");
        assert_eq!(
            answer.files_changed,
            vec!["a.lean".to_string(), "b.lean".to_string()]
        );
        assert_eq!(
            answer.files_created,
            vec!["a.lean".to_string(), "b.lean".to_string()]
        );
    }

    #[tokio::test]
    async fn test_unexpected_finish_reason_returns_error() {
        // 返回 "length" 这种未处理的 finish_reason
        let (agent, _tmp, _ws) = make_agent(vec![unexpected_response("length")]).await;

        let (events, _messages) = agent.run("触发异常", &[], false).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "error");
        assert!(
            events[0].content.contains("length"),
            "应提及意外的 finish_reason"
        );
        assert!(events[0].duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_max_iterations_terminates_with_error() {
        // 始终返回 tool_calls（用不存在的工具，避免副作用），超过 MAX_ITERATIONS
        let mut endless: Vec<ChatResponse> = Vec::new();
        for _ in 0..MAX_ITERATIONS {
            endless.push(tool_call_response("call_x", "nonexistent_tool", json!({})));
        }
        // 不再 push stop——期望在第 MAX_ITERATIONS 次后因达到上限终止
        let (agent, _tmp, _ws) = make_agent(endless).await;

        let (events, _messages) = agent.run("无限循环", &[], false).await.unwrap();

        let last = events.last().expect("应有事件");
        assert_eq!(last.kind, "error");
        assert!(
            last.content.contains("最大迭代次数"),
            "应提示达到最大迭代次数"
        );
        assert!(last.duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_thinking_event_emitted_when_reasoning_present() {
        // 带推理内容的 stop 响应
        let resp = ChatResponse {
            finish_reason: "stop".to_string(),
            content: Some("最终答案".to_string()),
            reasoning_content: Some("让我想想...".to_string()),
            tool_calls: vec![],
        };
        let (agent, _tmp, _ws) = make_agent(vec![resp]).await;

        let (events, _messages) = agent.run("思考", &[], false).await.unwrap();

        // 期望：thinking 事件 + answer 事件
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "thinking");
        assert_eq!(events[0].content, "让我想想...");
        assert_eq!(events[1].kind, "answer");
        assert_eq!(events[1].content, "最终答案");
    }

    #[tokio::test]
    async fn test_history_is_included_in_messages() {
        // 验证 history 被正确拼入 messages
        let (agent, _tmp, _ws) = make_agent(vec![stop_response("ok")]).await;

        let history = vec![
            json!({ "role": "user", "content": "上一轮问题" }),
            json!({ "role": "assistant", "content": "上一轮回答" }),
        ];

        let (_events, messages) = agent.run("本轮问题", &history, false).await.unwrap();

        // system + 2 history + user + assistant(stop 追加) = 5
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["content"], "上一轮问题");
        assert_eq!(messages[2]["content"], "上一轮回答");
        assert_eq!(messages[3]["role"], "user");
        assert_eq!(messages[3]["content"], "本轮问题");
        assert_eq!(messages[4]["role"], "assistant");
        assert_eq!(messages[4]["content"], "ok");
    }

    #[tokio::test]
    async fn test_tool_call_event_carries_parsed_args() {
        // 验证 tool_call 事件的 tool_args 被正确解析为 JSON
        let args = json!({ "path": "test.lean", "content": "lemma l : True := by trivial" });
        let (agent, _tmp, _ws) = make_agent(vec![
            tool_call_response("c1", "write_file", args.clone()),
            stop_response("done"),
        ])
        .await;

        let (events, _messages) = agent.run("写", &[], false).await.unwrap();

        let tool_call_ev = events.iter().find(|e| e.kind == "tool_call").unwrap();
        assert_eq!(tool_call_ev.tool_args, Some(args));
    }
}
