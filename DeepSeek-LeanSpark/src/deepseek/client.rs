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
