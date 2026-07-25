use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ProofStateTool;

impl Default for ProofStateTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofStateTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ProofStateTool {
    fn name(&self) -> &str {
        "get_proof_state"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "get_proof_state",
                "description": "从 Lean4 代码中提取证明状态信息。Phase 1 通过静态解析提取 theorem/lemma 声明、是否含 sorry、by 块数量。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lean_code": {
                            "type": "string",
                            "description": "Lean4 代码"
                        }
                    },
                    "required": ["lean_code"]
                }
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let code = args.get("lean_code").and_then(|v| v.as_str()).unwrap_or("");

        let mut declarations = Vec::new();
        for line in code.lines() {
            let trimmed = line.trim();
            let kind_and_rest = trimmed
                .strip_prefix("theorem ")
                .map(|r| ("theorem", r))
                .or_else(|| trimmed.strip_prefix("lemma ").map(|r| ("lemma", r)));
            if let Some((kind, rest)) = kind_and_rest {
                let name = rest.split_whitespace().next().unwrap_or("");
                if !name.is_empty() {
                    declarations.push(json!({ "kind": kind, "name": name }));
                }
            }
        }

        let has_sorry = code.contains("sorry") || code.contains("admit");
        let by_count = code.matches(" by ").count();

        Ok(serde_json::to_string(&json!({
            "success": true,
            "declarations": declarations,
            "has_sorry": has_sorry,
            "by_blocks_count": by_count,
            "note": "Phase 1 为静态解析。Phase 2 将通过 LSP 提供实时 goal state。"
        }))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    async fn call_tool(code: &str) -> Value {
        let tool = ProofStateTool::new();
        let args = json!({ "lean_code": code });
        let s = tool.call(&args).await.unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn spec_has_correct_name() {
        let tool = ProofStateTool::new();
        let spec = tool.spec();
        assert_eq!(spec["function"]["name"], "get_proof_state");
        assert_eq!(spec["function"]["parameters"]["required"][0], "lean_code");
    }

    #[tokio::test]
    async fn extracts_theorem_and_lemma_declarations() {
        let code = r#"
theorem add_comm (a b : Nat) : a + b = b + a := by
  sorry

lemma helper (x : Nat) : x + 0 = x := by
  rfl
"#;
        let v = call_tool(code).await;
        let decls = v["declarations"].as_array().unwrap();
        assert_eq!(decls.len(), 2, "应提取 2 个声明");
        assert_eq!(decls[0]["kind"], "theorem");
        assert_eq!(decls[0]["name"], "add_comm");
        assert_eq!(decls[1]["kind"], "lemma");
        assert_eq!(decls[1]["name"], "helper");
    }

    #[tokio::test]
    async fn detects_sorry_and_admit() {
        let with_sorry = call_tool("theorem t : True := by sorry").await;
        assert_eq!(with_sorry["has_sorry"], true);

        let with_admit = call_tool("theorem t : True := by admit").await;
        assert_eq!(with_admit["has_sorry"], true);

        let clean = call_tool("theorem t : True := by trivial").await;
        assert_eq!(clean["has_sorry"], false);
    }

    #[tokio::test]
    async fn counts_by_blocks() {
        let code = "theorem t : True := by trivial\ntheorem s : True := by rfl";
        let v = call_tool(code).await;
        assert_eq!(v["by_blocks_count"], 2, "应统计 2 个 by 块");
    }

    #[tokio::test]
    async fn empty_code_returns_empty_declarations() {
        let v = call_tool("").await;
        assert_eq!(v["success"], true);
        assert_eq!(v["declarations"].as_array().unwrap().len(), 0);
        assert_eq!(v["has_sorry"], false);
        assert_eq!(v["by_blocks_count"], 0);
    }

    #[tokio::test]
    async fn ignores_non_declaration_lines_with_theorem_substring() {
        // "theorem" 出现在注释或字符串中，但不是行首声明，不应被提取
        let code = "-- this is not a theorem\ntypeof_theorem : True := by trivial";
        let v = call_tool(code).await;
        let decls = v["declarations"].as_array().unwrap();
        assert_eq!(decls.len(), 0, "注释/非声明行的 theorem 不应被提取");
    }

    #[tokio::test]
    async fn handles_missing_lean_code_argument() {
        let tool = ProofStateTool::new();
        // proof_state 对缺失 lean_code 用 unwrap_or("")，不报错
        let args = json!({});
        let s = tool.call(&args).await.unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(v["declarations"].as_array().unwrap().len(), 0);
    }
}
