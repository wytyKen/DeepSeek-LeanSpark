use crate::lean::LeanRunner;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct LeanCheckTool {
    lean: Arc<LeanRunner>,
}

impl LeanCheckTool {
    pub fn new(lean: Arc<LeanRunner>) -> Self {
        Self { lean }
    }
}

#[async_trait]
impl Tool for LeanCheckTool {
    fn name(&self) -> &str {
        "run_lean_check"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "run_lean_check",
                "description": "提交 Lean4 代码给编译器验证。返回 {success, output, warning}。success=true 表示编译通过。",
                "strict": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lean_code": {
                            "type": "string",
                            "description": "完整的 Lean4 代码，包含所有 import 与待验证的 theorem/lemma。禁止使用 sorry 或 admit。"
                        }
                    },
                    "required": ["lean_code"],
                    "additionalProperties": false
                }
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let lean_code = args
            .get("lean_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing lean_code argument"))?;

        let result = self.lean.run(lean_code).await?;
        let warning = if result.contains_sorry {
            " [WARNING: 代码包含 sorry 或 admit，这违反安全规则——禁止使用 sorry 跳过证明。请用真实 tactic 完成证明。]"
        } else {
            ""
        };
        let resp = json!({
            "success": result.success,
            "output": result.output,
            "warning": warning
        });
        Ok(serde_json::to_string(&resp)?)
    }
}
