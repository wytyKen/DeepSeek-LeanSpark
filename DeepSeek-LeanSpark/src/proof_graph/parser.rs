// 证明依赖图解析（静态解析版）
//
// 从 Lean4 代码中提取 theorem/lemma 声明以及它们在证明中调用的其他引理/tactic，
// 构造一个有向图：节点 = 声明或被引用的引理，边 = "A 的证明调用了 B"。
//
// 限制：基于正则，无法理解 Lean4 语法细节（如缩进块、命名空间）。
// Phase 3 将接入 Lean4 LSP 提供精确依赖。

use anyhow::Result;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub kind: String,   // "theorem" | "lemma" | "external"
    pub external: bool, // 是否为外部引理（非当前文件声明）
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String, // 声明节点 id
    pub to: String,   // 被调用引理节点 id
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub note: String,
}

/// 内置 tactic 名（这些不是引理，跳过不画边）
const TACTIC_NAMES: &[&str] = &[
    "rfl",
    "simp",
    "ring",
    "norm_num",
    "decide",
    "rw",
    "rewrite",
    "exact",
    "apply",
    "induction",
    "cases",
    "use",
    "have",
    "let",
    "calc",
    "constructor",
    "rcases",
    "obtain",
    "refine",
    "by_contra",
    "contradiction",
    "tauto",
    "aesop",
    "linarith",
    "nlinarith",
    "omega",
    "assumption",
    "intros",
    "intro",
    "fun",
    "show",
    "split",
    "left",
    "right",
    "exfalso",
    "sorry",
    "admit",
    "simp only",
    "ext",
    "unfold",
    "dsimp",
];

pub fn parse(code: &str) -> Result<ProofGraph> {
    // 提取声明：`theorem NAME ...` 或 `lemma NAME ...`
    // 用多行模式 + 行首匹配
    let decl_re = Regex::new(r"(?m)^\s*(theorem|lemma)\s+([A-Za-z_][A-Za-z0-9_'.]*)")?;
    let mut declarations: Vec<(String, String)> = Vec::new(); // (kind, name)
    for cap in decl_re.captures_iter(code) {
        let kind = cap.get(1).unwrap().as_str().to_string();
        let name = cap.get(2).unwrap().as_str().to_string();
        declarations.push((kind, name));
    }

    // 为每个声明找出其证明体中调用的引理
    // 简化策略：以声明的"行号区间"划分证明体
    let lines: Vec<&str> = code.lines().collect();
    let mut decl_ranges: Vec<(usize, usize, String, String)> = Vec::new(); // (start, end, kind, name)
    for (idx, (kind, name)) in declarations.iter().enumerate() {
        let start = lines
            .iter()
            .position(|l| {
                l.contains(&format!("{} {}", kind, name))
                    && (l.trim_start().starts_with(kind.as_str()))
            })
            .unwrap_or(0);
        // end = 当前声明之后第一个声明的位置（避免 end < start 导致证明体为空）；
        //       若无后续声明，则取文件末尾。
        let end = declarations
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != idx)
            .filter_map(|(_, (k2, n2))| {
                lines.iter().position(|l| {
                    l.contains(&format!("{} {}", k2, n2))
                        && (l.trim_start().starts_with(k2.as_str()))
                })
            })
            .filter(|&p| p > start)
            .min()
            .unwrap_or(lines.len());
        decl_ranges.push((start, end, kind.clone(), name.clone()));
    }

    // 提取 tactic 调用中的引理名
    let apply_re = Regex::new(r"\bapply\s+([A-Za-z_][A-Za-z0-9_'.]*)")?;
    let exact_re = Regex::new(r"\bexact\s+([A-Za-z_][A-Za-z0-9_'.]*)")?;
    let have_re = Regex::new(r"\bhave\s+\w+\s*[:=].*?:=\s*([A-Za-z_][A-Za-z0-9_'.]*)")?;
    // rw [a, b, c] / simp [a, b, c] / simp only [a, b]
    let rw_re = Regex::new(r"\brw\s+\[([^\]]+)\]")?;
    let simp_re = Regex::new(r"\bsimp(?:\s+only)?\s+\[([^\]]+)\]")?;

    let mut nodes_map: HashMap<String, GraphNode> = HashMap::new();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    let local_names: BTreeSet<String> = declarations.iter().map(|(_, n)| n.clone()).collect();

    // 先把本地声明加入节点
    for (kind, name) in &declarations {
        nodes_map.insert(
            name.clone(),
            GraphNode {
                id: name.clone(),
                name: name.clone(),
                kind: kind.clone(),
                external: false,
            },
        );
    }

    let add_external = |nodes: &mut HashMap<String, GraphNode>, name: &str| {
        if TACTIC_NAMES.contains(&name) || name.is_empty() {
            return;
        }
        // 跳过数字、纯操作符
        if name
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            return;
        }
        nodes.entry(name.to_string()).or_insert_with(|| GraphNode {
            id: name.to_string(),
            name: name.to_string(),
            kind: "external".to_string(),
            external: true,
        });
    };

    for (start, end, _kind, decl_name) in &decl_ranges {
        let proof_body: String = lines
            .iter()
            .skip(*start)
            .take(end.saturating_sub(*start))
            .copied()
            .collect::<Vec<_>>()
            .join("\n");

        // apply X / exact X
        for re in [&apply_re, &exact_re, &have_re] {
            for cap in re.captures_iter(&proof_body) {
                let referenced = cap.get(1).unwrap().as_str().to_string();
                if local_names.contains(&referenced) {
                    edges.insert((decl_name.clone(), referenced.clone()));
                } else {
                    add_external(&mut nodes_map, &referenced);
                    edges.insert((decl_name.clone(), referenced));
                }
            }
        }

        // rw [a, b, c]
        for cap in rw_re.captures_iter(&proof_body) {
            let inner = cap.get(1).unwrap().as_str();
            for item in inner.split(',') {
                let name = item
                    .trim()
                    .trim_start_matches('?')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                if local_names.contains(&name) {
                    edges.insert((decl_name.clone(), name.clone()));
                } else {
                    add_external(&mut nodes_map, &name);
                    edges.insert((decl_name.clone(), name));
                }
            }
        }

        // simp [a, b, c] / simp only [a, b]
        for cap in simp_re.captures_iter(&proof_body) {
            let inner = cap.get(1).unwrap().as_str();
            for item in inner.split(',') {
                let name = item
                    .trim()
                    .trim_start_matches('?')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                if local_names.contains(&name) {
                    edges.insert((decl_name.clone(), name.clone()));
                } else {
                    add_external(&mut nodes_map, &name);
                    edges.insert((decl_name.clone(), name));
                }
            }
        }
    }

    let mut nodes: Vec<GraphNode> = nodes_map.into_values().collect();
    nodes.sort_by(|a, b| a.name.cmp(&b.name));

    let edges: Vec<GraphEdge> = edges
        .into_iter()
        .map(|(from, to)| GraphEdge { from, to })
        .collect();

    Ok(ProofGraph {
        nodes,
        edges,
        note: "静态解析版（基于正则），可能不完整。Phase 3 将接入 Lean4 LSP 提供精确依赖。"
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_theorem_with_apply() {
        let code = r#"
theorem foo : True := by
  apply True.intro

theorem bar : True := by
  apply foo
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
        assert!(names.contains(&"True.intro")); // external
                                                // foo -> True.intro
        assert!(g
            .edges
            .iter()
            .any(|e| e.from == "foo" && e.to == "True.intro"));
        // bar -> foo
        assert!(g.edges.iter().any(|e| e.from == "bar" && e.to == "foo"));
    }

    #[test]
    fn parses_rw_with_multiple_lemmas() {
        let code = r#"
theorem t : 1 + 1 = 2 := by
  rw [add_comm, Nat.add_one]
"#;
        let g = parse(code).unwrap();
        assert!(g.edges.iter().any(|e| e.from == "t" && e.to == "add_comm"));
        assert!(g
            .edges
            .iter()
            .any(|e| e.from == "t" && e.to == "Nat.add_one"));
    }

    #[test]
    fn ignores_tactic_names_as_nodes() {
        let code = r#"
theorem t : True := by
  simp
  exact True.intro
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        // simp/exact 是 tactic 名，不应作为节点
        assert!(!names.contains(&"simp"));
        assert!(!names.contains(&"exact"));
        assert!(names.contains(&"True.intro"));
    }

    // ============ E2 扩展测试 ============

    #[test]
    fn parses_empty_code_returns_empty_graph() {
        // 空代码应返回空图，不报错
        let g = parse("").unwrap();
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
        // note 字段始终存在
        assert!(!g.note.is_empty());
    }

    #[test]
    fn parses_invalid_syntax_returns_empty_graph() {
        // 非 Lean4 代码（纯文本）应返回空图，不报错
        let g = parse("this is not lean code at all").unwrap();
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn parses_namespaced_theorem_name() {
        // 命名空间限定的定理名（含点号）应被正确提取
        let code = r#"
theorem Foo.Bar.baz : True := by
  apply True.intro
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("baz")),
            "应识别命名空间限定的定理名，实际：{:?}",
            names
        );
    }

    #[test]
    fn parses_lemma_keyword() {
        // lemma 关键字声明的引理应被识别为 lemma 类型节点
        let code = r#"
lemma helper : True := by
  apply True.intro
"#;
        let g = parse(code).unwrap();
        let helper = g
            .nodes
            .iter()
            .find(|n| n.name == "helper")
            .expect("应识别 lemma 声明");
        assert_eq!(helper.kind, "lemma");
        assert!(!helper.external);
    }

    #[test]
    fn external_lemma_marked_correctly() {
        // 不在当前文件声明的引理（如 mathlib 引理）应标记为 external
        let code = r#"
theorem t : True := by
  apply Nat.add_comm
"#;
        let g = parse(code).unwrap();
        let ext = g
            .nodes
            .iter()
            .find(|n| n.name == "Nat.add_comm")
            .expect("应识别外部引理 Nat.add_comm");
        assert!(ext.external, "外部引理应标记 external=true");
        assert_eq!(ext.kind, "external");
    }

    #[test]
    fn no_duplicate_nodes_for_same_reference() {
        // 多次引用同一引理不应产生重复节点
        let code = r#"
theorem t1 : True := by
  apply True.intro

theorem t2 : True := by
  apply True.intro

theorem t3 : True := by
  apply True.intro
"#;
        let g = parse(code).unwrap();
        let count = g.nodes.iter().filter(|n| n.name == "True.intro").count();
        assert_eq!(count, 1, "True.intro 应只有一个节点");
    }

    #[test]
    fn no_duplicate_edges_for_same_call() {
        // 同一声明内多次调用同一引理只应产生一条边
        let code = r#"
theorem t : True := by
  apply True.intro
  apply True.intro
  apply True.intro
"#;
        let g = parse(code).unwrap();
        let edge_count = g
            .edges
            .iter()
            .filter(|e| e.from == "t" && e.to == "True.intro")
            .count();
        assert_eq!(edge_count, 1, "重复调用只应产生一条边");
    }

    #[test]
    fn parses_simp_only_with_lemmas() {
        // simp only [a, b, c] 应提取每个引理
        let code = r#"
theorem t : 0 = 0 := by
  simp only [Nat.zero_eq, Nat.add_zero]
"#;
        let g = parse(code).unwrap();
        assert!(
            g.edges
                .iter()
                .any(|e| e.from == "t" && e.to == "Nat.zero_eq"),
            "应识别 simp only 中的引理"
        );
        assert!(
            g.edges
                .iter()
                .any(|e| e.from == "t" && e.to == "Nat.add_zero"),
            "应识别 simp only 中的引理"
        );
    }

    #[test]
    fn parses_simp_with_lemmas() {
        // simp [a, b] 应提取每个引理
        let code = r#"
theorem t : True := by
  simp [Nat.succ_inj, Nat.add_one]
"#;
        let g = parse(code).unwrap();
        assert!(g
            .edges
            .iter()
            .any(|e| e.from == "t" && e.to == "Nat.succ_inj"));
        assert!(g
            .edges
            .iter()
            .any(|e| e.from == "t" && e.to == "Nat.add_one"));
    }

    #[test]
    fn parses_have_with_lemma_reference() {
        // have h : T := by apply foo 中的 foo 应被识别
        let code = r#"
theorem t : True := by
  have h : True := by apply True.intro
  exact h
"#;
        let g = parse(code).unwrap();
        // True.intro 应被引用（来自 have 块中的 apply）
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"True.intro"), "have 块中的引理引用应被识别");
    }

    #[test]
    fn ignores_sorry_as_node() {
        // sorry 是 tactic 名，不应作为节点
        let code = r#"
theorem hard : False := by
  sorry
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"hard"), "hard 声明应被识别");
        assert!(!names.contains(&"sorry"), "sorry 不应作为节点");
    }

    #[test]
    fn ignores_tactic_keywords_as_external() {
        // 内置 tactic 名（apply/exact/rfl 等）不应被误识别为外部引理
        let code = r#"
theorem t : True := by
  apply True.intro
  rfl
  induction
  simp
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(!names.contains(&"apply"), "apply 不应作为节点");
        assert!(!names.contains(&"rfl"), "rfl 不应作为节点");
        assert!(!names.contains(&"induction"), "induction 不应作为节点");
        assert!(!names.contains(&"simp"), "simp 不应作为节点");
    }

    #[test]
    fn cross_declaration_dependency() {
        // 后声明的定理引用前声明的定理应建立正确边
        let code = r#"
theorem base : True := by
  apply True.intro

theorem step : True := by
  apply base

theorem top : True := by
  apply step
"#;
        let g = parse(code).unwrap();
        // base -> True.intro
        assert!(g
            .edges
            .iter()
            .any(|e| e.from == "base" && e.to == "True.intro"));
        // step -> base
        assert!(g.edges.iter().any(|e| e.from == "step" && e.to == "base"));
        // top -> step
        assert!(g.edges.iter().any(|e| e.from == "top" && e.to == "step"));
    }

    #[test]
    fn multiple_theorems_dont_inherit_each_others_deps() {
        // 两个独立定理不应互相继承依赖
        let code = r#"
theorem a : True := by
  apply True.intro

theorem b : True := by
  apply Nat.succ_inj'
"#;
        let g = parse(code).unwrap();
        // a 不应依赖 Nat.succ_inj'
        assert!(
            !g.edges
                .iter()
                .any(|e| e.from == "a" && e.to == "Nat.succ_inj'"),
            "a 不应继承 b 的依赖"
        );
        // b 不应依赖 True.intro
        assert!(
            !g.edges
                .iter()
                .any(|e| e.from == "b" && e.to == "True.intro"),
            "b 不应继承 a 的依赖"
        );
    }

    #[test]
    fn nodes_are_sorted_by_name() {
        // 节点列表应按名字排序，便于稳定输出
        let code = r#"
theorem zeta : True := by apply True.intro
theorem alpha : True := by apply True.intro
theorem mu : True := by apply True.intro
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g
            .nodes
            .iter()
            .filter(|n| !n.external)
            .map(|n| n.name.as_str())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "节点应按名字排序");
    }

    #[test]
    fn graph_note_mentions_static_analysis_limitation() {
        // note 字段应说明这是静态解析，可能不完整
        let g = parse("theorem t : True := by trivial").unwrap();
        assert!(
            g.note.contains("静态解析") || g.note.contains("正则"),
            "note 应说明静态解析限制，实际：{}",
            g.note
        );
    }

    #[test]
    fn parses_dotted_name_in_apply() {
        // apply 后跟带点的引理名（如 List.length_cons）应被正确提取
        let code = r#"
theorem t : True := by
  apply List.length_cons
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(
            names.contains(&"List.length_cons"),
            "应识别带点的引理名，实际：{:?}",
            names
        );
    }

    #[test]
    fn parses_prime_suffixed_name() {
        // 引理名带撇号（如 Nat.succ_inj'）应被识别
        let code = r#"
theorem t : True := by
  apply Nat.succ_inj'
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(
            names.contains(&"Nat.succ_inj'"),
            "应识别带撇号的引理名，实际：{:?}",
            names
        );
    }

    #[test]
    fn does_not_misidentify_comment_lines() {
        // -- 注释行中的 theorem 字样不应被识别为声明
        let code = r#"
-- theorem fake : True := by apply True.intro
theorem real : True := by apply True.intro
"#;
        let g = parse(code).unwrap();
        let names: Vec<_> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(
            !names.contains(&"fake"),
            "注释中的 theorem 不应被识别，实际：{:?}",
            names
        );
        assert!(names.contains(&"real"), "真实声明应被识别");
    }
}
