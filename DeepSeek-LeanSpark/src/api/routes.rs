use crate::AppState;
use axum::{
    extract::{self, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/lean/check", post(lean_check))
        .route("/api/tools", get(list_tools))
        .route("/api/models", get(list_models))
        // 工作区
        .route("/api/workspace/open", post(workspace_open))
        .route("/api/workspace/current", get(workspace_current))
        .route("/api/workspace/close", post(workspace_close))
        .route("/api/workspace/tree", get(workspace_tree))
        .route("/api/workspace/read", post(workspace_read))
        .route("/api/workspace/write", post(workspace_write))
        // 证明依赖图
        .route("/api/proof-graph", post(proof_graph))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    history: Vec<Value>,
    #[serde(default)]
    thinking: bool,
}

#[derive(Serialize)]
struct ChatResponseDto {
    events: Vec<crate::agent::AgentEvent>,
    messages: Vec<Value>,
}

async fn chat(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<ChatRequest>,
) -> Result<Json<ChatResponseDto>, (axum::http::StatusCode, String)> {
    let (events, messages) = state
        .agent
        .run(&req.message, &req.history, req.thinking)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ChatResponseDto { events, messages }))
}

#[derive(Deserialize)]
struct LeanCheckRequest {
    code: String,
}

async fn lean_check(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<LeanCheckRequest>,
) -> Result<Json<crate::lean::LeanResult>, (axum::http::StatusCode, String)> {
    let result = state
        .lean
        .run(&req.code)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(result))
}

async fn list_tools(State(state): State<AppState>) -> Json<Vec<Value>> {
    Json(state.tools.specs())
}

async fn list_models(State(state): State<AppState>) -> Json<Value> {
    Json(serde_json::json!({
        "current": state.deepseek.model(),
        "candidates": ["deepseek-v4-pro", "deepseek-v4-flash"]
    }))
}

// ============ 工作区路由 ============

#[derive(Deserialize)]
struct WorkspaceOpenRequest {
    path: String,
}

#[derive(Serialize)]
struct WorkspaceCurrentDto {
    open: bool,
    path: Option<String>,
    tree: Option<crate::workspace::FileNode>,
}

async fn workspace_open(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<WorkspaceOpenRequest>,
) -> Result<Json<WorkspaceCurrentDto>, (axum::http::StatusCode, String)> {
    state
        .workspace
        .open(&req.path)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
    let tree = state
        .workspace
        .list_tree()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(WorkspaceCurrentDto {
        open: true,
        path: Some(req.path),
        tree,
    }))
}

async fn workspace_current(State(state): State<AppState>) -> Json<WorkspaceCurrentDto> {
    let path = state.workspace.current().await;
    let tree = state.workspace.list_tree().await.ok().flatten();
    Json(WorkspaceCurrentDto {
        open: path.is_some(),
        path: path.map(|p| p.to_string_lossy().to_string()),
        tree,
    })
}

async fn workspace_close(State(state): State<AppState>) -> Json<Value> {
    state.workspace.close().await;
    Json(serde_json::json!({ "success": true }))
}

async fn workspace_tree(
    State(state): State<AppState>,
) -> Result<Json<Option<crate::workspace::FileNode>>, (axum::http::StatusCode, String)> {
    let tree = state
        .workspace
        .list_tree()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(tree))
}

#[derive(Deserialize)]
struct WorkspaceReadRequest {
    path: String,
}

#[derive(Serialize)]
struct WorkspaceReadDto {
    success: bool,
    path: String,
    content: Option<String>,
    error: Option<String>,
}

async fn workspace_read(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<WorkspaceReadRequest>,
) -> Json<WorkspaceReadDto> {
    match state.workspace.read_file(&req.path).await {
        Ok((content, _)) => Json(WorkspaceReadDto {
            success: true,
            path: req.path,
            content: Some(content),
            error: None,
        }),
        Err(e) => Json(WorkspaceReadDto {
            success: false,
            path: req.path,
            content: None,
            error: Some(e.to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct WorkspaceWriteRequest {
    path: String,
    content: String,
}

async fn workspace_write(
    State(state): State<AppState>,
    extract::Json(req): extract::Json<WorkspaceWriteRequest>,
) -> Json<Value> {
    match state.workspace.write_file(&req.path, &req.content).await {
        Ok(r) => Json(serde_json::json!({
            "success": true,
            "path": r.path,
            "created": r.created,
            "bytes": r.bytes
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "path": req.path,
            "error": e.to_string()
        })),
    }
}

// ============ 证明依赖图 ============

#[derive(Deserialize)]
struct ProofGraphRequest {
    code: String,
}

async fn proof_graph(
    extract::Json(req): extract::Json<ProofGraphRequest>,
) -> Result<Json<crate::proof_graph::ProofGraph>, (axum::http::StatusCode, String)> {
    let g = crate::proof_graph::parse(&req.code)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(g))
}
