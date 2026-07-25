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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentLoop, AppState, DeepSeekClient, LeanRunner, WorkspaceManager};
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    /// 构造测试用 AppState（用空 API key 和不存在的 lean 二进制）。
    fn make_state() -> AppState {
        let deepseek = Arc::new(DeepSeekClient::new(
            "test-key".to_string(),
            "test-model".to_string(),
        ));
        let lean = Arc::new(LeanRunner::new("/nonexistent/lean".to_string()));
        let workspace = Arc::new(WorkspaceManager::new());
        let tools = Arc::new(crate::tools::ToolRegistry::new_with_workspace(
            lean.clone(),
            workspace.clone(),
        ));
        let agent = Arc::new(AgentLoop::new(deepseek.clone(), tools.clone()));
        AppState {
            deepseek,
            lean,
            tools,
            agent,
            workspace,
        }
    }

    async fn body_str(body: Body) -> String {
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        String::from_utf8_lossy(&bytes).to_string()
    }

    // ---------------- /api/health ----------------

    #[tokio::test]
    async fn health_returns_ok() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_str(resp.into_body()).await;
        assert_eq!(body, "ok");
    }

    // ---------------- 404 ----------------

    #[tokio::test]
    async fn nonexistent_route_returns_404() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/this-does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "未知路由应返回 404");
    }

    // ---------------- 方法错误 ----------------

    #[tokio::test]
    async fn get_on_post_route_returns_method_not_allowed() {
        // /api/chat 只接受 POST，用 GET 应返回 405
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/chat")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "对 POST 路由用 GET 应返回 405"
        );
    }

    #[tokio::test]
    async fn post_on_get_route_returns_method_not_allowed() {
        // /api/health 只接受 GET，用 POST 应返回 405
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "对 GET 路由用 POST 应返回 405"
        );
    }

    // ---------------- 请求体校验（422） ----------------

    #[tokio::test]
    async fn chat_with_missing_message_returns_422() {
        let app = router(make_state());
        // 缺少 message 字段
        let body = serde_json::json!({ "history": [], "thinking": false }).to_string();
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "缺少 message 字段应返回 422"
        );
    }

    #[tokio::test]
    async fn chat_with_empty_body_returns_422() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn chat_with_invalid_json_returns_400() {
        // axum 的 Json extractor 行为：
        // - content-type 非 application/json → 415
        // - content-type 是 application/json 但 body 不是合法 JSON → 400
        // - JSON 合法但反序列化失败（缺必填字段）→ 422
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json-at-all"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "非法 JSON 应返回 400"
        );
    }

    #[tokio::test]
    async fn workspace_open_with_missing_path_returns_422() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/workspace/open")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn proof_graph_with_missing_code_returns_422() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/proof-graph")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ---------------- 成功路径（不需要外部资源） ----------------

    #[tokio::test]
    async fn workspace_current_when_closed_returns_open_false() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/workspace/current")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_str(resp.into_body()).await;
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["open"], false, "未打开工作区时 open 应为 false");
        assert!(v["path"].is_null(), "未打开工作区时 path 应为 null");
    }

    #[tokio::test]
    async fn list_tools_returns_array() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/tools")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_str(resp.into_body()).await;
        let v: Value = serde_json::from_str(&body).unwrap();
        let arr = v.as_array().expect("/api/tools 应返回数组");
        // 至少注册了 lean_check / search / proof_state / read_file / write_file
        assert!(arr.len() >= 5, "应至少注册 5 个工具，实际: {}", arr.len());
        let names: Vec<&str> = arr
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap_or(""))
            .collect();
        assert!(names.contains(&"run_lean_check"), "应含 run_lean_check");
        assert!(names.contains(&"read_file"), "应含 read_file");
        assert!(names.contains(&"write_file"), "应含 write_file");
    }

    #[tokio::test]
    async fn list_models_returns_current_and_candidates() {
        let app = router(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_str(resp.into_body()).await;
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["current"], "test-model");
        let candidates = v["candidates"].as_array().unwrap();
        assert!(candidates.len() >= 2, "应返回至少 2 个候选模型");
    }

    #[tokio::test]
    async fn proof_graph_with_valid_code_returns_200() {
        let app = router(make_state());
        let body = serde_json::json!({
            "code": "theorem add_comm (a b : Nat) : a + b = b + a := by sorry\nlemma helper : True := by trivial"
        })
        .to_string();
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/proof-graph")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_str(resp.into_body()).await;
        let v: Value = serde_json::from_str(&body).unwrap();
        // 应至少有 2 个节点（add_comm, helper）
        let nodes = v["nodes"].as_array().expect("nodes 应为数组");
        assert!(nodes.len() >= 2, "应至少有 2 个节点，实际: {}", nodes.len());
    }

    #[tokio::test]
    async fn workspace_open_with_nonexistent_path_returns_400() {
        let app = router(make_state());
        let body = serde_json::json!({ "path": "/nonexistent/path/xyz" }).to_string();
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/workspace/open")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "打开不存在的路径应返回 400"
        );
    }
}
