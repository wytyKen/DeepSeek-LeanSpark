pub mod agent;
pub mod api;
pub mod deepseek;
pub mod lean;
pub mod proof_graph;
pub mod tools;
pub mod workspace;

pub use agent::AgentLoop;
pub use deepseek::DeepSeekClient;
pub use lean::LeanRunner;
pub use workspace::WorkspaceManager;

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub deepseek: Arc<DeepSeekClient>,
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

    let api_key =
        std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set (see .env.example)");
    let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
    let lean_path = std::env::var("LEAN_BIN_PATH").unwrap_or_else(|_| "lean".to_string());
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let deepseek = Arc::new(DeepSeekClient::new(api_key, model));
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
