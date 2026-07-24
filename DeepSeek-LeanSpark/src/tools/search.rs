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
