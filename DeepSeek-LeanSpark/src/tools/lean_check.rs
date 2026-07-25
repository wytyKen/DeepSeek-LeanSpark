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
        let resp = format_result(&result);
        Ok(serde_json::to_string(&resp)?)
    }
}

/// 把 LeanResult 格式化为返回给 LLM 的 JSON 值。
/// 抽取为独立函数便于单测（无需 mock LeanRunner）。
fn format_result(result: &crate::lean::LeanResult) -> Value {
    let warning = if result.contains_sorry {
        " [WARNING: 代码包含 sorry 或 admit，这违反安全规则——禁止使用 sorry 跳过证明。请用真实 tactic 完成证明。]"
    } else {
        ""
    };
    json!({
        "success": result.success,
        "output": result.output,
        "warning": warning
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lean::LeanResult;

    fn make_result(success: bool, contains_sorry: bool, output: &str) -> LeanResult {
        LeanResult {
            success,
            output: output.to_string(),
            error: if success {
                None
            } else {
                Some(output.to_string())
            },
            contains_sorry,
        }
    }

    #[test]
    fn spec_has_correct_name_and_parameters() {
        let tool = LeanCheckTool::new(Arc::new(LeanRunner::new("lean".to_string())));
        let spec = tool.spec();
        assert_eq!(spec["function"]["name"], "run_lean_check");
        assert_eq!(spec["function"]["parameters"]["required"][0], "lean_code");
        assert_eq!(
            spec["function"]["parameters"]["properties"]["lean_code"]["type"],
            "string"
        );
    }

    #[test]
    fn format_result_success_without_sorry() {
        let r = make_result(true, false, "no errors");
        let v = format_result(&r);
        assert_eq!(v["success"], true);
        assert_eq!(v["output"], "no errors");
        assert_eq!(v["warning"], "");
    }

    #[test]
    fn format_result_success_with_sorry_adds_warning() {
        let r = make_result(true, true, "no errors");
        let v = format_result(&r);
        assert_eq!(v["success"], true);
        assert!(
            v["warning"].as_str().unwrap().contains("sorry"),
            "warning 应提及 sorry"
        );
        assert!(
            v["warning"].as_str().unwrap().contains("安全规则"),
            "warning 应提及安全规则"
        );
    }

    #[test]
    fn format_result_failure_without_sorry() {
        let r = make_result(false, false, "type mismatch");
        let v = format_result(&r);
        assert_eq!(v["success"], false);
        assert_eq!(v["output"], "type mismatch");
        assert_eq!(v["warning"], "", "无 sorry 时不应有 warning");
    }

    #[test]
    fn format_result_failure_with_sorry_still_warns() {
        let r = make_result(false, true, "sorry detected");
        let v = format_result(&r);
        assert_eq!(v["success"], false);
        assert!(
            v["warning"].as_str().unwrap().contains("sorry"),
            "即使编译失败，含 sorry 仍应警告"
        );
    }

    #[tokio::test]
    async fn call_missing_lean_code_returns_error() {
        let tool = LeanCheckTool::new(Arc::new(LeanRunner::new("lean".to_string())));
        let args = json!({}); // 缺少 lean_code
        let result = tool.call(&args).await;
        assert!(result.is_err(), "缺少 lean_code 应返回 Err");
        assert!(
            result.unwrap_err().to_string().contains("lean_code"),
            "错误信息应提及 lean_code"
        );
    }

    #[tokio::test]
    async fn call_with_nonexistent_lean_binary_returns_failure_response() {
        // 用不存在的 lean 二进制路径，run() 会返回 success=false 的 LeanResult（不 panic）
        // 注意：runner 把 IO 错误填到 LeanResult.error 字段，但 format_result 只序列化了 output
        // 所以这里只能断言 success=false 与 output 非空（output 在失败场景为 stderr 或空）
        let tool = LeanCheckTool::new(Arc::new(LeanRunner::new(
            "/nonexistent/lean_binary_xyz".to_string(),
        )));
        let args = json!({ "lean_code": "theorem t : True := by trivial" });
        let result_str = tool.call(&args).await.unwrap();
        let v: Value = serde_json::from_str(&result_str).unwrap();
        assert_eq!(
            v["success"], false,
            "lean 二进制不存在时应返回 success=false"
        );
        // output 字段类型必须是 string（即使是空字符串）
        assert!(
            v.get("output").and_then(|o| o.as_str()).is_some(),
            "output 字段应为 string 类型"
        );
    }
}
