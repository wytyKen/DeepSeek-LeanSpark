// tests/workspace_smoke.rs
// 运行：cargo test --test workspace_smoke
// 前置：先启动后端（cargo run），监听 127.0.0.1:3000

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const BASE: &str = "http://127.0.0.1:3000";

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

/// 构造临时工作区目录，返回其绝对路径
fn make_tmp_workspace() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("leanspark_ws_{}", uuid::Uuid::new_v4()));
    fs::create_dir(&dir).unwrap();
    // 放几个测试文件
    fs::write(dir.join("a.lean"), "theorem t : True := by trivial\n").unwrap();
    fs::write(dir.join("readme.md"), "# test workspace\n").unwrap();
    fs::create_dir(dir.join("subdir")).unwrap();
    fs::write(
        dir.join("subdir").join("b.lean"),
        "theorem u : True := by trivial\n",
    )
    .unwrap();
    dir
}

#[test]
fn workspace_open_and_list_tree() {
    let dir = make_tmp_workspace();
    let resp = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    assert!(
        resp.status().is_success(),
        "open failed: {:?}",
        resp.status()
    );
    let v: Value = resp.json().unwrap();
    assert_eq!(v["open"], true);
    let tree = v["tree"].as_object().expect("tree must be object");
    assert_eq!(tree["kind"], "dir");
    let children = tree["children"].as_array().expect("children must be array");
    let names: Vec<String> = children
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"a.lean".to_string()));
    assert!(names.contains(&"readme.md".to_string()));
    assert!(names.contains(&"subdir".to_string()));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_read_file_within_root() {
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/read", BASE))
        .json(&serde_json::json!({ "path": "readme.md" }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true);
    assert!(v["content"].as_str().unwrap().contains("test workspace"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_read_file_escape_root_rejected() {
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    // 试图通过 .. 路径穿越读取工作区外的文件
    let resp = client()
        .post(format!("{}/api/workspace/read", BASE))
        .json(&serde_json::json!({ "path": "../escape.txt" }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);
    // 错误信息应明确提及超出工作区
    let err = v["error"].as_str().unwrap();
    assert!(
        err.contains("超出") || err.contains("escapes") || err.contains("escape"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_write_file_creates_new() {
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/write", BASE))
        .json(&serde_json::json!({
            "path": "proofs/new.lean",
            "content": "theorem t : True := by trivial\n"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["created"], true);
    // 验证文件实际存在
    assert!(dir.join("proofs").join("new.lean").exists());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_write_file_escape_root_rejected() {
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/write", BASE))
        .json(&serde_json::json!({
            "path": "../escape.lean",
            "content": "x"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);
    let err = v["error"].as_str().unwrap();
    assert!(
        err.contains("超出") || err.contains("escapes") || err.contains("escape"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn proof_graph_parses_simple_theorem() {
    let code = r#"theorem foo : True := by
  apply True.intro

theorem bar : True := by
  apply foo
"#;
    let resp = client()
        .post(format!("{}/api/proof-graph", BASE))
        .json(&serde_json::json!({ "code": code }))
        .send()
        .unwrap();
    assert!(resp.status().is_success());
    let v: Value = resp.json().unwrap();
    let nodes = v["nodes"].as_array().unwrap();
    let names: Vec<String> = nodes
        .iter()
        .map(|n| n["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"bar".to_string()));
    assert!(names.contains(&"True.intro".to_string()));
    let edges = v["edges"].as_array().unwrap();
    assert!(edges
        .iter()
        .any(|e| e["from"] == "foo" && e["to"] == "True.intro"));
    assert!(edges.iter().any(|e| e["from"] == "bar" && e["to"] == "foo"));
}

#[test]
fn tools_list_includes_read_write_file() {
    // 默认启动后 ToolRegistry 已注册 read_file/write_file（启动时一次性注册）
    let resp = client().get(format!("{}/api/tools", BASE)).send().unwrap();
    assert!(resp.status().is_success());
    let arr: Vec<Value> = resp.json().unwrap();
    let names: Vec<String> = arr
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"read_file".to_string()));
    assert!(names.contains(&"write_file".to_string()));
    // 原有工具仍应存在
    assert!(names.contains(&"run_lean_check".to_string()));
    assert!(names.contains(&"search_mathlib".to_string()));
    assert!(names.contains(&"get_proof_state".to_string()));
}

// ============ 边界与错误路径测试（E1 扩展） ============

#[test]
fn workspace_open_nonexistent_path_rejected() {
    // 打开一个不存在的路径应被拒绝
    let resp = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": "/definitely/not/exist/leanspark_test_xyz" }))
        .send()
        .unwrap();
    assert!(!resp.status().is_success(), "应返回 4xx 错误");
}

#[test]
fn workspace_open_file_not_directory_rejected() {
    // 打开一个文件（非目录）应被拒绝
    let tmp_file =
        std::env::temp_dir().join(format!("leanspark_file_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&tmp_file, "not a directory").unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": tmp_file.to_string_lossy() }))
        .send()
        .unwrap();
    assert!(!resp.status().is_success(), "应返回 4xx：路径不是目录");
    fs::remove_file(&tmp_file).unwrap();
}

#[test]
fn workspace_read_nonexistent_file_returns_error() {
    // 读取工作区内不存在的文件，应返回 success=false 与错误信息
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/read", BASE))
        .json(&serde_json::json!({ "path": "nonexistent.lean" }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);
    assert!(!v["error"].as_str().unwrap().is_empty());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_write_to_subdirectory_creates_dirs() {
    // 写入嵌套子目录的文件，应自动创建父目录链
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/write", BASE))
        .json(&serde_json::json!({
          "path": "proofs/chapter1/exercises/ex1.lean",
          "content": "theorem t : True := by trivial\n"
        }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["created"], true);
    // 验证文件与父目录链都存在
    assert!(dir
        .join("proofs")
        .join("chapter1")
        .join("exercises")
        .join("ex1.lean")
        .exists());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_close_clears_state() {
    // 关闭工作区后，current 应返回 open=false
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let _ = client()
        .post(format!("{}/api/workspace/close", BASE))
        .send()
        .unwrap();
    let resp = client()
        .get(format!("{}/api/workspace/current", BASE))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["open"], false);
    assert!(v["path"].is_null());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_read_after_close_rejected() {
    // 关闭工作区后读取文件应失败
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let _ = client()
        .post(format!("{}/api/workspace/close", BASE))
        .send()
        .unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/read", BASE))
        .json(&serde_json::json!({ "path": "a.lean" }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_tree_excludes_target_and_hidden() {
    // 文件树应排除 target/、.git/、.lake/ 等目录与隐藏文件
    let dir = make_tmp_workspace();
    // 添加应被排除的目录与文件
    fs::create_dir(dir.join("target")).unwrap();
    fs::write(dir.join("target").join("build.artifact"), "should not show").unwrap();
    fs::create_dir(dir.join(".git")).unwrap();
    fs::write(dir.join(".git").join("config"), "should not show").unwrap();
    fs::create_dir(dir.join(".lake")).unwrap();
    fs::write(dir.join(".hidden.lean"), "should not show").unwrap();

    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    let resp = client()
        .get(format!("{}/api/workspace/tree", BASE))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    let serialized = serde_json::to_string(&v).unwrap();
    assert!(!serialized.contains("target"), "target/ 不应在文件树中");
    assert!(!serialized.contains(".git"), ".git/ 不应在文件树中");
    assert!(!serialized.contains(".lake"), ".lake/ 不应在文件树中");
    assert!(
        !serialized.contains(".hidden.lean"),
        "隐藏文件不应在文件树中"
    );
    // 应保留正常文件
    assert!(serialized.contains("a.lean"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_write_overwrite_existing_file() {
    // 写入已存在的文件，created 应为 false
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    // 第一次写：created=true
    let resp1 = client()
        .post(format!("{}/api/workspace/write", BASE))
        .json(&serde_json::json!({
          "path": "proofs/exist.lean",
          "content": "theorem a : True := by trivial\n"
        }))
        .send()
        .unwrap();
    let v1: Value = resp1.json().unwrap();
    assert_eq!(v1["success"], true);
    assert_eq!(v1["created"], true);

    // 第二次写同一文件：created=false
    let resp2 = client()
        .post(format!("{}/api/workspace/write", BASE))
        .json(&serde_json::json!({
          "path": "proofs/exist.lean",
          "content": "theorem b : True := by trivial\n"
        }))
        .send()
        .unwrap();
    let v2: Value = resp2.json().unwrap();
    assert_eq!(v2["success"], true);
    assert_eq!(v2["created"], false);

    // 验证文件内容被覆盖
    let resp3 = client()
        .post(format!("{}/api/workspace/read", BASE))
        .json(&serde_json::json!({ "path": "proofs/exist.lean" }))
        .send()
        .unwrap();
    let v3: Value = resp3.json().unwrap();
    assert!(v3["content"].as_str().unwrap().contains("theorem b"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_read_absolute_path_rejected() {
    // 用绝对路径尝试读取工作区外文件应被拒绝
    let dir = make_tmp_workspace();
    let _ = client()
        .post(format!("{}/api/workspace/open", BASE))
        .json(&serde_json::json!({ "path": dir.to_string_lossy() }))
        .send()
        .unwrap();
    // 构造工作区外的绝对路径
    let outside =
        std::env::temp_dir().join(format!("leanspark_outside_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&outside, "secret").unwrap();
    let resp = client()
        .post(format!("{}/api/workspace/read", BASE))
        .json(&serde_json::json!({ "path": outside.to_string_lossy() }))
        .send()
        .unwrap();
    let v: Value = resp.json().unwrap();
    assert_eq!(v["success"], false);

    fs::remove_file(&outside).unwrap();
    fs::remove_dir_all(&dir).unwrap();
}
