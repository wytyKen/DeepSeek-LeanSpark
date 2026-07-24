use crate::deepseek::{ChatResponse, DeepSeekClient};
use crate::tools::ToolRegistry;
use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

const MAX_ITERATIONS: usize = 20;

pub struct AgentLoop {
    client: Arc<DeepSeekClient>,
    tools: Arc<ToolRegistry>,
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
    pub fn new(client: Arc<DeepSeekClient>, tools: Arc<ToolRegistry>) -> Self {
        Self { client, tools }
    }

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
