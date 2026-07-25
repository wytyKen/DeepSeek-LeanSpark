pub mod agent;
pub mod api;
pub mod deepseek;
pub mod lean;
pub mod proof_graph;
pub mod tools;
pub mod workspace;

pub use agent::AgentLoop;
pub use deepseek::{DeepSeekClient, SharedChatClient};
pub use lean::LeanRunner;
pub use workspace::WorkspaceManager;

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub deepseek: Arc<SharedChatClient>,
    pub lean: Arc<LeanRunner>,
    pub tools: Arc<tools::ToolRegistry>,
    pub agent: Arc<AgentLoop>,
    pub workspace: Arc<WorkspaceManager>,
}

pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    // 用 try_init() 而非 init()：当被 Tauri 桌面壳调用时，
    // main.rs 已经初始化过全局 subscriber，这里再次 init 会 panic。
    // try_init() 在已初始化时返回 Err，我们忽略即可。
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();

    // API Key 延迟初始化（Phase 2 关键修复）：
    // - 旧实现用 expect("DEEPSEEK_API_KEY must be set")，未配置 key 时 Tauri 应用直接 panic，
    //   用户无法进入设置界面。
    // - 新实现用 ok() 容错：未配置 key 时 SharedChatClient::new(None)，应用可正常启动；
    //   用户通过 UI 设置 key 后调 replace_client 注入真实客户端。
    //   .env 仍是 Web 形态开发者配置 key 的入口（dotenvy::dotenv 已加载）。
    let api_key = std::env::var("DEEPSEEK_API_KEY").ok();
    let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
    let lean_path = std::env::var("LEAN_BIN_PATH").unwrap_or_else(|_| "lean".to_string());
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let shared_client = match api_key {
        Some(key) if !key.is_empty() => {
            tracing::info!("DeepSeek API Key 已从环境变量加载，模型：{}", model);
            SharedChatClient::new(Some(DeepSeekClient::new(key, model)))
        }
        _ => {
            tracing::warn!(
                "DEEPSEEK_API_KEY 未配置或为空，应用以「未配置」状态启动。\
                 请通过 UI 设置界面填入 API Key 后再发起对话。"
            );
            SharedChatClient::new(None)
        }
    };
    let deepseek = Arc::new(shared_client);
    let lean = Arc::new(LeanRunner::new(lean_path));
    let workspace = Arc::new(WorkspaceManager::new());
    // 启动时先注册基础工具，工作区工具在工作区打开后通过 ToolRegistry::register_workspace_tools 注册
    // 但 ToolRegistry 内部用 Arc，注册新工具需要可变访问——这里用一次性全注册策略：
    // 直接把 WorkspaceManager 也注入，工具内部检查 workspace 是否打开。
    let tools = Arc::new(tools::ToolRegistry::new_with_workspace(
        lean.clone(),
        workspace.clone(),
    ));
    let agent = Arc::new(AgentLoop::new(deepseek.clone(), tools.clone()));

    let state = AppState {
        deepseek,
        lean,
        tools,
        agent,
        workspace,
    };

    let app = api::routes::router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
