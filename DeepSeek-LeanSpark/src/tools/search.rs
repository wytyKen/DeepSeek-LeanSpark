use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct SearchMathlibTool {
    common_lemmas: Vec<(&'static str, &'static str)>,
}

impl Default for SearchMathlibTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchMathlibTool {
    pub fn new() -> Self {
        Self {
            // Phase 1：内置常用引理索引。Phase 3 将接入完整 mathlib 搜索。
            common_lemmas: vec![
                ("add_comm", "a + b = b + a"),
                ("add_assoc", "(a + b) + c = a + (b + c)"),
                ("add_zero", "a + 0 = a"),
                ("zero_add", "0 + a = a"),
                ("mul_comm", "a * b = b * a"),
                ("mul_assoc", "(a * b) * c = a * (b * c)"),
                ("mul_one", "a * 1 = a"),
                ("one_mul", "1 * a = a"),
                ("add_left_cancel", "a + b = a + c → b = c"),
                ("add_right_cancel", "b + a = c + a → b = c"),
                ("le_refl", "a ≤ a"),
                ("le_trans", "a ≤ b → b ≤ c → a ≤ c"),
                ("lt_of_le_of_lt", "a ≤ b → b < c → a < c"),
                ("eq_refl", "a = a"),
                ("Ne.symm", "a ≠ b → b ≠ a"),
                ("Iff.refl", "P ↔ P"),
                ("True.intro", "True"),
                ("False.elim", "False → P"),
                ("Classical.by_contradiction", "¬¬P → P"),
                ("by_contra", "反证法 tactic"),
                ("Nat.succ_eq_add_one", "n.succ = n + 1"),
                ("Nat.add_zero", "n + 0 = n"),
                ("Nat.zero_add", "0 + n = n"),
                ("Nat.mul_one", "n * 1 = n"),
                ("Nat.one_mul", "1 * n = n"),
                ("Nat.le_refl", "n ≤ n"),
                ("Continuous.comp", "连续函数复合连续"),
                ("Continuous.add", "两连续函数之和连续"),
                ("Continuous.mul", "两连续函数之积连续"),
                ("Continuous.neg", "连续函数取负连续"),
                ("Continuous.sub", "两连续函数之差连续"),
                ("Real.continuous_pow", "实数幂函数连续"),
                ("Real.continuous_abs", "绝对值连续"),
                ("Monotone.add", "两单调函数之和单调"),
                ("Monotone.mul_of_nonneg", "非负系数下单调"),
                ("Filter.tendsto_add", "极限的和等于和的极限"),
                ("Filter.tendsto_mul", "极限的积等于积的极限"),
                ("tendsto_const", "常数列收敛到自身"),
                ("Finset.sum_add_distrib", "求和分配到加法"),
                ("Finset.sum_mul", "求和与乘法"),
                ("Finset.card_union", "并集基数"),
                ("Set.union_comm", "集合并交换律"),
                ("Set.inter_comm", "集合交交换律"),
                ("Set.subset_def", "子集定义"),
                ("List.map", "列表映射"),
                // 常用 tactic
                ("ring", "环等式 tactic"),
                ("norm_num", "数值规范化 tactic"),
                ("decide", "决策过程 tactic"),
                ("simp", "化简器 tactic"),
                ("rw", "重写 tactic"),
                ("rewrite", "重写 tactic"),
                ("exact", "精确匹配 tactic"),
                ("apply", "应用 tactic"),
                ("induction", "归纳 tactic"),
                ("cases", "分情况 tactic"),
                ("use", "提供见证 tactic"),
                ("have", "引入中间结论 tactic"),
                ("let", "引入绑定 tactic"),
                ("calc", "计算证明 tactic"),
                ("constructor", "构造器 tactic"),
                ("rcases", "递归分情况 tactic"),
                ("obtain", "获取 tactic"),
                ("refine", "细化 tactic"),
                ("simp only", "只化简指定引理 tactic"),
            ],
        }
    }
}

#[async_trait]
impl Tool for SearchMathlibTool {
    fn name(&self) -> &str {
        "search_mathlib"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "search_mathlib",
                "description": "搜索 mathlib 中的定理/引理/tactic 名称及其简述。Phase 1 提供常见引理索引。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索关键词，例如 'continuous'、'add_comm'、'monotone'"
                        }
                    },
                    "required": ["query"]
                }
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let matches: Vec<_> = self
            .common_lemmas
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&query))
            .collect();

        if matches.is_empty() {
            Ok(serde_json::to_string(&json!({
                "success": false,
                "message": format!(
                    "未找到匹配 '{}' 的引理。建议尝试关键词: continuous, add, mul, monotone, comm, assoc, le, lt, tendsto, simp, rw, ring",
                    query
                ),
            }))?)
        } else {
            let results: Vec<Value> = matches
                .iter()
                .map(|(name, desc)| json!({ "name": name, "description": desc }))
                .collect();
            Ok(serde_json::to_string(&json!({
                "success": true,
                "results": results,
                "note": "Phase 1 仅提供常见引理索引。Phase 3 将接入完整 mathlib 全文搜索。"
            }))?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    async fn call_tool(query: &str) -> Value {
        let tool = SearchMathlibTool::new();
        let args = json!({ "query": query });
        let s = tool.call(&args).await.unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn spec_has_correct_name() {
        let tool = SearchMathlibTool::new();
        let spec = tool.spec();
        assert_eq!(spec["function"]["name"], "search_mathlib");
        assert_eq!(spec["function"]["parameters"]["required"][0], "query");
    }

    #[tokio::test]
    async fn search_add_returns_addition_lemmas() {
        let v = call_tool("add").await;
        assert_eq!(v["success"], true);
        let results = v["results"].as_array().unwrap();
        let names: Vec<&str> = results
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"add_comm"), "应含 add_comm");
        assert!(names.contains(&"add_assoc"), "应含 add_assoc");
        assert!(names.contains(&"add_zero"), "应含 add_zero");
        assert!(names.contains(&"zero_add"), "应含 zero_add");
    }

    #[tokio::test]
    async fn search_continuous_returns_continuous_lemmas() {
        let v = call_tool("continuous").await;
        assert_eq!(v["success"], true);
        let results = v["results"].as_array().unwrap();
        assert!(!results.is_empty(), "continuous 应有匹配");
        let names: Vec<&str> = results
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Continuous.comp"), "应含 Continuous.comp");
        assert!(names.contains(&"Continuous.add"), "应含 Continuous.add");
    }

    #[tokio::test]
    async fn search_nonexistent_returns_failure_with_suggestions() {
        let v = call_tool("zzz_nonexistent_xyz").await;
        assert_eq!(v["success"], false);
        let msg = v["message"].as_str().unwrap();
        assert!(msg.contains("未找到"), "应提示未找到");
        assert!(msg.contains("建议"), "应给建议关键词");
    }

    #[tokio::test]
    async fn search_is_case_insensitive() {
        // 小写 "continuous" 与大写 "Continuous" 都应匹配
        let lower = call_tool("continuous").await;
        let upper = call_tool("Continuous").await;
        let lower_count = lower["results"].as_array().unwrap().len();
        let upper_count = upper["results"].as_array().unwrap().len();
        assert_eq!(lower_count, upper_count, "大小写不敏感应返回相同数量");
    }

    #[tokio::test]
    async fn search_empty_query_returns_all() {
        // 空字符串 query，所有 name 都 contains("")，返回全部
        let v = call_tool("").await;
        assert_eq!(v["success"], true);
        let count = v["results"].as_array().unwrap().len();
        assert!(count > 10, "空 query 应返回大量内置引理");
    }

    #[tokio::test]
    async fn search_tactic_keywords() {
        let v = call_tool("ring").await;
        assert_eq!(v["success"], true);
        let names: Vec<&str> = v["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"ring"), "应含 ring tactic");
    }

    #[tokio::test]
    async fn search_missing_query_argument_uses_empty_string() {
        // query 缺失时 unwrap_or("") → 返回全部（不报错）
        let tool = SearchMathlibTool::new();
        let args = json!({});
        let s = tool.call(&args).await.unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["success"], true);
    }
}
