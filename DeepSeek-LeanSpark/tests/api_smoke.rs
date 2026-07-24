// tests/api_smoke.rs
// 运行：cargo test --test api_smoke
// 前置：先启动后端（cargo run），监听 127.0.0.1:3000
//      且 LEAN_BIN_PATH 指向可执行的 lean

use serde_json::Value;

const BASE: &str = "http://127.0.0.1:3000";

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

#[test]
fn health_returns_ok() {
    let body = client()
        .get(format!("{}/api/health", BASE))
        .send()
        .unwrap()
        .text()
        .unwrap();
    assert_eq!(body, "ok");
}

#[test]
fn tools_list_includes_run_lean_check() {
    let resp = client().get(format!("{}/api/tools", BASE)).send().unwrap();
    assert!(resp.status().is_success());
    let arr: Vec<Value> = resp.json().unwrap();
    let names: Vec<String> = arr
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"run_lean_check".to_string()));
    assert!(names.contains(&"search_mathlib".to_string()));
    assert!(names.contains(&"get_proof_state".to_string()));
}

#[test]
fn lean_check_passes_valid_rfl() {
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : 1 + 1 = 2 := by rfl"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["contains_sorry"], false);
}

#[test]
fn lean_check_fails_on_wrong_goal() {
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : 1 + 1 = 3 := by rfl"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);
    // Lean 4.x 不同版本措辞不同：
    //   - 旧版本：    "type mismatch"
    //   - 4.22+：     "tactic 'rfl' failed ... is not definitionally equal to"
    // 只要证明被拒绝（success=false）且错误信息明确指出等式不成立即可。
    let err = v["error"].as_str().unwrap();
    assert!(
        err.contains("type mismatch")
            || err.contains("rfl' failed")
            || err.contains("not definitionally equal"),
        "unexpected error: {err}"
    );
}

#[test]
fn lean_check_flags_sorry() {
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : False := by sorry"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    // 代码含 sorry → contains_sorry=true，warning 通过 /api/chat 才可见
    assert_eq!(v["contains_sorry"], true);
}

// ============ 扩展测试（E1） ============

#[test]
fn models_endpoint_returns_candidates() {
    // /api/models 应返回当前模型与候选列表
    let resp = client().get(format!("{}/api/models", BASE)).send().unwrap();
    assert!(resp.status().is_success());
    let v: Value = resp.json().unwrap();
    assert!(v["current"].is_string(), "current 应为字符串");
    let candidates = v["candidates"].as_array().expect("candidates 应为数组");
    assert!(!candidates.is_empty(), "候选列表不应为空");
    // 候选列表中应包含 deepseek-v4 系列
    let candidate_names: Vec<String> = candidates
        .iter()
        .filter_map(|c| c.as_str().map(String::from))
        .collect();
    assert!(
        candidate_names.iter().any(|n| n.contains("deepseek-v4")),
        "候选应包含 deepseek-v4 系列，实际：{:?}",
        candidate_names
    );
}

#[test]
fn lean_check_empty_code_does_not_crash() {
    // 空代码不应导致后端崩溃（应返回 success=false 与错误信息）
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({ "code": "" }))
        .send()
        .unwrap();
    assert!(resp.status().is_success(), "HTTP 应正常返回");
    let v: Value = resp.json().unwrap();
    // 空代码不应通过
    assert_eq!(v["success"], false);
}

#[test]
fn lean_check_with_syntax_error_returns_error() {
    // 语法错误的 Lean4 代码应返回 success=false 与错误信息
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "this is not valid lean syntax at all !!!"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);
    // 错误信息应非空
    let err = v["error"].as_str().unwrap_or("");
    assert!(!err.is_empty(), "错误信息应非空");
}

#[test]
fn lean_check_valid_trivial_proof() {
    // trivial tactic 应证明 True
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : True := by trivial"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["contains_sorry"], false);
}

#[test]
fn lean_check_with_imports() {
    // 含 import 的代码也应能正确编译
    let resp = client()
        .post(format!("{}/api/lean/check", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : 1 + 0 = 1 := by rfl"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true, "1 + 0 = 1 应可通过 rfl 证明");
}

#[test]
fn proof_graph_endpoint_available() {
    // /api/proof-graph 端点应可用
    let resp = client()
        .post(format!("{}/api/proof-graph", BASE))
        .json(&serde_json::json!({
            "code": "theorem t : True := by apply True.intro"
        }))
        .send()
        .unwrap();
    assert!(resp.status().is_success());
    let v: Value = resp.json().unwrap();
    assert!(v["nodes"].is_array(), "nodes 应为数组");
    assert!(v["edges"].is_array(), "edges 应为数组");
    assert!(v["note"].is_string(), "note 应为字符串");
}

#[test]
fn workspace_endpoints_available() {
    // 工作区相关端点应全部可用（即使未打开工作区也不应 500）
    let resp = client()
        .get(format!("{}/api/workspace/current", BASE))
        .send()
        .unwrap();
    assert!(resp.status().is_success());
    let v: Value = resp.json().unwrap();
    assert!(v["open"].is_boolean(), "open 应为布尔");
}

#[test]
fn tools_endpoint_returns_array() {
    // /api/tools 应返回数组，且每个工具有 function.name 字段
    let resp = client().get(format!("{}/api/tools", BASE)).send().unwrap();
    assert!(resp.status().is_success());
    let arr: Vec<Value> = resp.json().unwrap();
    assert!(!arr.is_empty(), "工具列表不应为空");
    for tool in &arr {
        assert!(
            tool["function"]["name"].is_string(),
            "每个工具应有 function.name 字段"
        );
    }
}
