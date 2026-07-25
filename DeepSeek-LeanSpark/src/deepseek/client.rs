use anyhow::Result;
use serde_json::Value;
use std::time::Duration;

const BASE_URL: &str = "https://api.deepseek.com";

#[derive(Clone)]
pub struct DeepSeekClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .expect("failed to build reqwest client");
        Self {
            client,
            api_key,
            model,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// 普通模式聊天补全
    pub async fn chat(
        &self,
        messages: &[Value],
        tools: Option<&[Value]>,
        tool_choice: Option<&str>,
    ) -> Result<ChatResponse> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });
        if let Some(t) = tools {
            body["tools"] = Value::Array(t.to_vec());
        }
        if let Some(tc) = tool_choice {
            body["tool_choice"] = Value::String(tc.to_string());
        }
        self.call(body).await
    }

    /// 思考模式聊天补全
    /// thinking 模式下回传 assistant 消息必须含 reasoning_content
    pub async fn chat_with_thinking(
        &self,
        messages: &[Value],
        tools: &[Value],
        reasoning_effort: &str,
    ) -> Result<ChatResponse> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "reasoning_effort": reasoning_effort,
            "thinking": { "type": "enabled" }
        });
        self.call(body).await
    }

    async fn call(&self, body: Value) -> Result<ChatResponse> {
        let resp = self
            .client
            .post(format!("{}/chat/completions", BASE_URL))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("DeepSeek API error {}: {}", status, text);
        }
        let value: Value = serde_json::from_str(&text)?;
        ChatResponse::from_value(value)
    }
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub finish_reason: String,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // 原始 JSON 字符串
}

impl ChatResponse {
    pub fn from_value(v: Value) -> Result<Self> {
        let choice = v
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| anyhow::anyhow!("missing choices[0] in response: {}", v))?;
        let message = choice
            .get("message")
            .ok_or_else(|| anyhow::anyhow!("missing message"))?;
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("stop")
            .to_string();
        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let reasoning_content = message
            .get("reasoning_content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tool_calls = message
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let name = tc.get("function")?.get("name")?.as_str()?.to_string();
                        let args = tc.get("function")?.get("arguments")?.as_str()?.to_string();
                        Some(ToolCall {
                            id,
                            name,
                            arguments: args,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            finish_reason,
            content,
            reasoning_content,
            tool_calls,
        })
    }

    /// 转换为可推回 messages 的 assistant 消息
    /// 关键：thinking 模式下回传必须含 reasoning_content，否则 400
    pub fn to_assistant_message(&self) -> Value {
        let mut msg = serde_json::json!({
            "role": "assistant",
            "content": self.content.clone(),
        });
        if let Some(rc) = &self.reasoning_content {
            msg["reasoning_content"] = Value::String(rc.clone());
        }
        if !self.tool_calls.is_empty() {
            msg["tool_calls"] = Value::Array(
                self.tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments
                            }
                        })
                    })
                    .collect(),
            );
        }
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---------------- ChatResponse::from_value 测试 ----------------

    #[test]
    fn from_value_parses_simple_stop_response() {
        let v = json!({
            "choices": [{
                "message": { "content": "证明完成" },
                "finish_reason": "stop"
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert_eq!(r.finish_reason, "stop");
        assert_eq!(r.content.as_deref(), Some("证明完成"));
        assert!(r.reasoning_content.is_none());
        assert!(r.tool_calls.is_empty());
    }

    #[test]
    fn from_value_parses_response_with_reasoning_content() {
        let v = json!({
            "choices": [{
                "message": {
                    "content": "最终答案",
                    "reasoning_content": "我先思考..."
                },
                "finish_reason": "stop"
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert_eq!(r.content.as_deref(), Some("最终答案"));
        assert_eq!(r.reasoning_content.as_deref(), Some("我先思考..."));
    }

    #[test]
    fn from_value_parses_tool_calls() {
        let v = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_abc",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"foo.lean\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert_eq!(r.finish_reason, "tool_calls");
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].id, "call_abc");
        assert_eq!(r.tool_calls[0].name, "read_file");
        assert_eq!(r.tool_calls[0].arguments, "{\"path\":\"foo.lean\"}");
    }

    #[test]
    fn from_value_parses_multiple_tool_calls() {
        let v = json!({
            "choices": [{
                "message": {
                    "tool_calls": [
                        { "id": "1", "function": { "name": "a", "arguments": "{}" } },
                        { "id": "2", "function": { "name": "b", "arguments": "{}" } }
                    ]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert_eq!(r.tool_calls.len(), 2);
        assert_eq!(r.tool_calls[0].name, "a");
        assert_eq!(r.tool_calls[1].name, "b");
    }

    #[test]
    fn from_value_skips_malformed_tool_calls() {
        // tool_calls 数组中某项缺 id / function.name / arguments 应被跳过
        let v = json!({
            "choices": [{
                "message": {
                    "tool_calls": [
                        { "id": "ok", "function": { "name": "good", "arguments": "{}" } },
                        { "id": "missing_name", "function": { "arguments": "{}" } },
                        { "function": { "name": "missing_id", "arguments": "{}" } },
                        { "id": "missing_args", "function": { "name": "no_args" } }
                    ]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert_eq!(r.tool_calls.len(), 1, "只有 1 个完整的 tool_call 应被解析");
        assert_eq!(r.tool_calls[0].name, "good");
    }

    #[test]
    fn from_value_handles_null_content() {
        // OpenAI/DeepSeek 在 tool_calls 场景下 content 常为 null
        let v = json!({
            "choices": [{
                "message": { "content": null },
                "finish_reason": "tool_calls"
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert!(r.content.is_none(), "null content 应映射为 None");
    }

    #[test]
    fn from_value_defaults_finish_reason_to_stop_when_missing() {
        let v = json!({
            "choices": [{
                "message": { "content": "x" }
            }]
        });
        let r = ChatResponse::from_value(v).unwrap();
        assert_eq!(r.finish_reason, "stop", "缺失 finish_reason 应默认为 stop");
    }

    #[test]
    fn from_value_errors_when_choices_missing() {
        let v = json!({ "error": "bad request" });
        let r = ChatResponse::from_value(v);
        assert!(r.is_err(), "缺失 choices 应返回 Err");
        assert!(
            r.unwrap_err().to_string().contains("choices"),
            "错误信息应提及 choices"
        );
    }

    #[test]
    fn from_value_errors_when_choices_empty_array() {
        let v = json!({ "choices": [] });
        let r = ChatResponse::from_value(v);
        assert!(r.is_err(), "空 choices 数组应返回 Err（无法取 choices[0]）");
    }

    #[test]
    fn from_value_errors_when_message_missing() {
        let v = json!({ "choices": [{ "finish_reason": "stop" }] });
        let r = ChatResponse::from_value(v);
        assert!(r.is_err(), "缺失 message 应返回 Err");
        assert!(
            r.unwrap_err().to_string().contains("message"),
            "错误信息应提及 message"
        );
    }

    // ---------------- ChatResponse::to_assistant_message 测试 ----------------

    #[test]
    fn to_assistant_message_for_simple_stop() {
        let r = ChatResponse {
            finish_reason: "stop".to_string(),
            content: Some("hello".to_string()),
            reasoning_content: None,
            tool_calls: vec![],
        };
        let m = r.to_assistant_message();
        assert_eq!(m["role"], "assistant");
        assert_eq!(m["content"], "hello");
        assert!(m.get("reasoning_content").is_none());
        assert!(m.get("tool_calls").is_none());
    }

    #[test]
    fn to_assistant_message_preserves_reasoning_content_for_thinking_mode() {
        // thinking 模式下回传必须含 reasoning_content，否则 DeepSeek API 返回 400
        let r = ChatResponse {
            finish_reason: "stop".to_string(),
            content: Some("answer".to_string()),
            reasoning_content: Some("thinking...".to_string()),
            tool_calls: vec![],
        };
        let m = r.to_assistant_message();
        assert_eq!(
            m["reasoning_content"], "thinking...",
            "reasoning_content 必须带回"
        );
    }

    #[test]
    fn to_assistant_message_includes_tool_calls_when_present() {
        let r = ChatResponse {
            finish_reason: "tool_calls".to_string(),
            content: None,
            reasoning_content: None,
            tool_calls: vec![ToolCall {
                id: "call_1".to_string(),
                name: "read_file".to_string(),
                arguments: "{\"path\":\"a.lean\"}".to_string(),
            }],
        };
        let m = r.to_assistant_message();
        let tcs = m["tool_calls"].as_array().unwrap();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0]["id"], "call_1");
        assert_eq!(tcs[0]["type"], "function");
        assert_eq!(tcs[0]["function"]["name"], "read_file");
        assert_eq!(tcs[0]["function"]["arguments"], "{\"path\":\"a.lean\"}");
    }

    #[test]
    fn to_assistant_message_with_null_content_and_tool_calls() {
        // tool_calls 场景下 content 通常为 None（null）
        let r = ChatResponse {
            finish_reason: "tool_calls".to_string(),
            content: None,
            reasoning_content: None,
            tool_calls: vec![ToolCall {
                id: "x".to_string(),
                name: "t".to_string(),
                arguments: "{}".to_string(),
            }],
        };
        let m = r.to_assistant_message();
        assert_eq!(m["content"], Value::Null);
        assert!(m.get("tool_calls").is_some());
    }

    // ---------------- DeepSeekClient 构造测试 ----------------

    #[test]
    fn new_client_preserves_api_key_and_model() {
        let c = DeepSeekClient::new("sk-test-key".to_string(), "deepseek-chat".to_string());
        assert_eq!(c.model(), "deepseek-chat");
        // api_key 是私有字段，通过行为间接验证：构造成功即可
    }

    #[test]
    fn new_client_with_empty_api_key_still_constructs() {
        // 边界：空 api_key 不应 panic（实际请求会被服务端拒绝）
        let c = DeepSeekClient::new("".to_string(), "deepseek-reasoner".to_string());
        assert_eq!(c.model(), "deepseek-reasoner");
    }
}
