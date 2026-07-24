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
