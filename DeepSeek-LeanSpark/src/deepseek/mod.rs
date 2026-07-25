mod client;

use anyhow::Result;
pub use client::{ChatResponse, DeepSeekClient, ToolCall};
use serde_json::Value;

/// 聊天客户端抽象：AgentLoop 依赖此 trait 而非具体 DeepSeekClient，便于单测 mock。
#[async_trait::async_trait]
pub trait ChatClient: Send + Sync {
    async fn chat(
        &self,
        messages: &[Value],
        tools: Option<&[Value]>,
        tool_choice: Option<&str>,
    ) -> Result<ChatResponse>;
    async fn chat_with_thinking(
        &self,
        messages: &[Value],
        tools: &[Value],
        reasoning_effort: &str,
    ) -> Result<ChatResponse>;
}

#[async_trait::async_trait]
impl ChatClient for DeepSeekClient {
    async fn chat(
        &self,
        messages: &[Value],
        tools: Option<&[Value]>,
        tool_choice: Option<&str>,
    ) -> Result<ChatResponse> {
        // 调用同名 inherent 方法（用 Fully-Qualified Syntax 消歧）
        DeepSeekClient::chat(self, messages, tools, tool_choice).await
    }
    async fn chat_with_thinking(
        &self,
        messages: &[Value],
        tools: &[Value],
        reasoning_effort: &str,
    ) -> Result<ChatResponse> {
        DeepSeekClient::chat_with_thinking(self, messages, tools, reasoning_effort).await
    }
}
