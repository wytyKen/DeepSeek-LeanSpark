// 可运行时替换的 ChatClient 包装。
//
// 设计目的：
// - Tauri 桌面应用首次启动时，用户尚未配置 DeepSeek API Key。
//   若直接用 `expect("DEEPSEEK_API_KEY must be set")` 会让整个应用 panic，
//   无法进入设置界面。本包装把客户端改为延迟初始化：
//   启动时 inner = None，所有 chat 调用返回友好错误；
//   用户在 UI 设置 API Key 后，调用 `replace_client` 注入真实客户端。
//
// - Web 形态下若 .env 已配置 key，启动时 inner = Some(client)，行为不变。
//
// - AgentLoop 依赖 `Arc<dyn ChatClient>`，本包装实现该 trait，
//   因此无需修改 AgentLoop 即可注入。

use std::sync::{Arc, RwLock};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{ChatClient, ChatResponse, DeepSeekClient};

const NOT_CONFIGURED_MSG: &str =
    "DeepSeek API Key 未配置。请在「设置」中填入 API Key 后再发起对话。";

/// 可运行时替换的 ChatClient。
#[derive(Clone)]
pub struct SharedChatClient {
    inner: Arc<RwLock<Option<DeepSeekClient>>>,
}

impl SharedChatClient {
    /// 创建包装。传入 None 表示尚未配置 API Key。
    pub fn new(client: Option<DeepSeekClient>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(client)),
        }
    }

    /// 替换内部客户端（用户在 UI 设置 API Key 后调用）。
    pub fn replace_client(&self, client: DeepSeekClient) {
        let mut guard = self.inner.write().expect("SharedChatClient lock poisoned");
        *guard = Some(client);
    }

    /// 当前是否已配置有效客户端。
    pub fn is_configured(&self) -> bool {
        self.inner
            .read()
            .expect("SharedChatClient lock poisoned")
            .is_some()
    }

    /// 当前模型名。未配置时返回默认模型 `deepseek-v4-flash`。
    /// 返回 String 而非 &str，避免持有 RwLock 读锁。
    pub fn model(&self) -> String {
        self.inner
            .read()
            .expect("SharedChatClient lock poisoned")
            .as_ref()
            .map(|c| c.model().to_string())
            .unwrap_or_else(|| "deepseek-v4-flash".to_string())
    }
}

#[async_trait]
impl ChatClient for SharedChatClient {
    async fn chat(
        &self,
        messages: &[Value],
        tools: Option<&[Value]>,
        tool_choice: Option<&str>,
    ) -> Result<ChatResponse> {
        // 注意：std::sync::RwLockReadGuard 不是 Send，不能跨 .await 持有。
        // 这里在 await 前先 clone 出 DeepSeekClient（DeepSeekClient 是 Clone），
        // 然后 drop guard，再调用 chat。
        let client = {
            let guard = self.inner.read().expect("SharedChatClient lock poisoned");
            guard.as_ref().cloned()
        };
        match client {
            Some(c) => c.chat(messages, tools, tool_choice).await,
            None => Err(anyhow::anyhow!(NOT_CONFIGURED_MSG)),
        }
    }

    async fn chat_with_thinking(
        &self,
        messages: &[Value],
        tools: &[Value],
        reasoning_effort: &str,
    ) -> Result<ChatResponse> {
        let client = {
            let guard = self.inner.read().expect("SharedChatClient lock poisoned");
            guard.as_ref().cloned()
        };
        match client {
            Some(c) => {
                c.chat_with_thinking(messages, tools, reasoning_effort)
                    .await
            }
            None => Err(anyhow::anyhow!(NOT_CONFIGURED_MSG)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 构造一个会被立即丢弃的占位客户端（仅用于测试未配置路径）
    fn make_unconfigured() -> SharedChatClient {
        SharedChatClient::new(None)
    }

    /// 构造一个已配置的 SharedChatClient
    fn make_configured(key: &str, model: &str) -> SharedChatClient {
        SharedChatClient::new(Some(DeepSeekClient::new(
            key.to_string(),
            model.to_string(),
        )))
    }

    #[test]
    fn unconfigured_is_configured_returns_false() {
        let c = make_unconfigured();
        assert!(!c.is_configured(), "未配置时 is_configured 应为 false");
    }

    #[test]
    fn configured_is_configured_returns_true() {
        let c = make_configured("sk-test", "deepseek-chat");
        assert!(c.is_configured(), "已配置时 is_configured 应为 true");
    }

    #[test]
    fn unconfigured_model_returns_default() {
        let c = make_unconfigured();
        assert_eq!(
            c.model(),
            "deepseek-v4-flash",
            "未配置时 model 应返回默认值"
        );
    }

    #[test]
    fn configured_model_returns_inner_model() {
        let c = make_configured("sk-test", "deepseek-reasoner");
        assert_eq!(c.model(), "deepseek-reasoner");
    }

    #[tokio::test]
    async fn unconfigured_chat_returns_friendly_error() {
        let c = make_unconfigured();
        let msgs = vec![json!({ "role": "user", "content": "hello" })];
        let result = c.chat(&msgs, None, None).await;
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("未配置"),
            "未配置时 chat 应返回含「未配置」的错误，实际: {}",
            err
        );
    }

    #[tokio::test]
    async fn unconfigured_chat_with_thinking_returns_friendly_error() {
        let c = make_unconfigured();
        let msgs = vec![json!({ "role": "user", "content": "hello" })];
        let tools: Vec<Value> = vec![];
        let result = c.chat_with_thinking(&msgs, &tools, "high").await;
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("未配置"),
            "未配置时 chat_with_thinking 应返回含「未配置」的错误，实际: {}",
            err
        );
    }

    #[test]
    fn replace_client_swaps_inner() {
        let c = make_unconfigured();
        assert!(!c.is_configured());
        assert_eq!(c.model(), "deepseek-v4-flash");

        // 注入真实客户端
        c.replace_client(DeepSeekClient::new(
            "sk-new".to_string(),
            "deepseek-reasoner".to_string(),
        ));
        assert!(c.is_configured(), "替换后应已配置");
        assert_eq!(c.model(), "deepseek-reasoner");
    }

    #[test]
    fn clone_shares_inner_state() {
        // Clone 后两个实例共享同一内部状态（Arc<RwLock<...>>）
        let c1 = make_configured("sk-test", "deepseek-chat");
        let c2 = c1.clone();
        assert!(c2.is_configured());

        // 通过 c1 替换为未配置（replace 只能注入 Some，无法清空，这里测替换）
        c1.replace_client(DeepSeekClient::new(
            "sk-other".to_string(),
            "deepseek-reasoner".to_string(),
        ));
        // c2 应看到模型变化
        assert_eq!(c2.model(), "deepseek-reasoner", "clone 应共享内部状态");
    }
}
