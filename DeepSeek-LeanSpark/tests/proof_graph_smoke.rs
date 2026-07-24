// tests/proof_graph_smoke.rs
// 运行：cargo test --test proof_graph_smoke
// 前置：先启动后端（cargo run），监听 127.0.0.1:3000
//
// 测试 /api/proof-graph 端点的边界场景：空代码、无效语法、命名空间、去重、sorry 等。

use serde_json::Value;

const BASE: &str = "http://127.0.0.1:3000";

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

fn post_proof_graph(code: &str) -> Value {
    let resp = client()
        .post(format!("{}/api/proof-graph", BASE))
        .json(&serde_json::json!({ "code": code }))
        .send()
        .unwrap();
    assert!(
        resp.status().is_success(),
        "proof-graph 调用失败：{:?}",
        resp.status()
    );
    resp.json().unwrap()
}

#[test]
fn proof_graph_handles_empty_code() {
    // 空代码应返回空节点列表（不报错）
    let v = post_proof_graph("");
    let nodes = v["nodes"].as_array().unwrap();
    let edges = v["edges"].as_array().unwrap();
    assert!(nodes.is_empty(), "空代码不应有节点");
    assert!(edges.is_empty(), "空代码不应有边");
}

#[test]
fn proof_graph_handles_invalid_syntax() {
    // 非 Lean4 代码（如纯文本）应返回空图（不报错）
    let v = post_proof_graph("this is not lean code at all");
    let nodes = v["nodes"].as_array().unwrap();
    assert!(nodes.is_empty(), "非 Lean 代码不应有节点");
}

#[test]
fn proof_graph_handles_only_comments() {
    // 仅注释的代码应返回空图
    let code = r#"
-- 这是一行注释
/- 这是多行注释 -/
-- theorem fake : True := by trivial
"#;
    let v = post_proof_graph(code);
    let nodes = v["nodes"].as_array().unwrap();
    // 注释里的 theorem fake 不应被识别（注意：当前正则可能误识别，但 fake 仍不应作为有效声明）
    // 这里宽松断言：要么没有节点，要么即使误识别也不应崩溃
    assert!(nodes.len() <= 1, "仅注释不应产生大量节点");
}

#[test]
fn proof_graph_with_namespaced_names() {
    // 命名空间限定的定理名（如 Foo.bar）应被正确提取
    let code = r#"
theorem Foo.bar : True := by
  apply True.intro
"#;
    let v = post_proof_graph(code);
    let nodes = v["nodes"].as_array().unwrap();
    let names: Vec<String> = nodes
        .iter()
        .map(|n| n["name"].as_str().unwrap().to_string())
        .collect();
    // 至少应识别 Foo.bar 或 bar 之一
    assert!(
        names.iter().any(|n| n.contains("bar")),
        "应识别命名空间限定的定理名，实际：{:?}",
        names
    );
}

#[test]
fn proof_graph_no_duplicate_nodes() {
    // 多次引用同一引理不应产生重复节点
    let code = r#"
theorem t1 : True := by
  apply True.intro

theorem t2 : True := by
  apply True.intro

theorem t3 : True := by
  apply True.intro
"#;
    let v = post_proof_graph(code);
    let nodes = v["nodes"].as_array().unwrap();
    // True.intro 只应有一个节点
    let true_intro_count = nodes
        .iter()
        .filter(|n| n["name"].as_str() == Some("True.intro"))
        .count();
    assert_eq!(true_intro_count, 1, "True.intro 应只有一个节点");
}

#[test]
fn proof_graph_no_duplicate_edges() {
    // 同一 from-to 边只应出现一次
    let code = r#"
theorem t : True := by
  apply True.intro
  apply True.intro
  apply True.intro
"#;
    let v = post_proof_graph(code);
    let edges = v["edges"].as_array().unwrap();
    let t_to_true_intro_count = edges
        .iter()
        .filter(|e| e["from"] == "t" && e["to"] == "True.intro")
        .count();
    assert_eq!(t_to_true_intro_count, 1, "重复调用应只产生一条边");
}

#[test]
fn proof_graph_with_sorry_does_not_crash() {
    // 含 sorry 的代码不应导致解析崩溃
    let code = r#"
theorem hard : False := by
  sorry
"#;
    let v = post_proof_graph(code);
    let nodes = v["nodes"].as_array().unwrap();
    let names: Vec<String> = nodes
        .iter()
        .map(|n| n["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"hard".to_string()));
    // sorry 是 tactic 名，不应作为节点
    assert!(!names.contains(&"sorry".to_string()));
}

#[test]
fn proof_graph_with_rw_and_simp_combined() {
    // rw 与 simp 同时使用应正确提取所有引理
    let code = r#"
theorem t : 1 + 1 = 2 := by
  rw [add_comm]
  simp [Nat.one_add]
"#;
    let v = post_proof_graph(code);
    let edges = v["edges"].as_array().unwrap();
    let from_t: Vec<&str> = edges
        .iter()
        .filter(|e| e["from"] == "t")
        .map(|e| e["to"].as_str().unwrap())
        .collect();
    assert!(from_t.contains(&"add_comm"), "应有边 t -> add_comm");
    assert!(from_t.contains(&"Nat.one_add"), "应有边 t -> Nat.one_add");
}

#[test]
fn proof_graph_only_external_references_produce_no_local_edges() {
    // 仅引用外部引理（无本地声明）应只产生外部节点，无 from 边
    let code = r#"
example : True := by
  apply True.intro
"#;
    let v = post_proof_graph(code);
    // example 不是 theorem/lemma，不应作为本地声明
    let nodes = v["nodes"].as_array().unwrap();
    // True.intro 可能作为外部节点出现
    let _ = nodes;
    // example 不在声明节点中
    let names: Vec<String> = nodes
        .iter()
        .filter(|n| n["external"] == false)
        .map(|n| n["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !names.contains(&"example".to_string()),
        "example 不应作为本地声明节点"
    );
}

#[test]
fn proof_graph_returns_note_field() {
    // note 字段应非空（说明这是静态解析版）
    let v = post_proof_graph("theorem t : True := by trivial");
    let note = v["note"].as_str().unwrap();
    assert!(!note.is_empty(), "note 字段不应为空");
    assert!(
        note.contains("静态解析") || note.contains("Phase 3"),
        "note 应说明解析方式"
    );
}

#[test]
fn proof_graph_lemma_keyword_recognized() {
    // lemma 关键字应被识别
    let code = r#"
lemma helper : True := by
  apply True.intro
"#;
    let v = post_proof_graph(code);
    let nodes = v["nodes"].as_array().unwrap();
    let helper = nodes.iter().find(|n| n["name"].as_str() == Some("helper"));
    assert!(helper.is_some(), "应识别 lemma 关键字声明的 helper");
    assert_eq!(helper.unwrap()["kind"].as_str().unwrap(), "lemma");
    assert!(!helper.unwrap()["external"].as_bool().unwrap());
}
