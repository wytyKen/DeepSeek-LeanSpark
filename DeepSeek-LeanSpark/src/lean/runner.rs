use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone)]
pub struct LeanRunner {
    lean_bin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub contains_sorry: bool,
}

impl LeanRunner {
    pub fn new(lean_bin: String) -> Self {
        Self { lean_bin }
    }

    /// 写入临时文件，执行 `lean <file>`，返回编译结果
    pub async fn run(&self, code: &str) -> Result<LeanResult> {
        let tmp = std::env::temp_dir().join(format!("leanspark_{}.lean", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp, code).await?;

        let output = Command::new(&self.lean_bin).arg(&tmp).output().await;

        // 清理临时文件（即使执行失败）
        let _ = tokio::fs::remove_file(&tmp).await;

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                return Ok(LeanResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "failed to execute lean: {}. Is '{}' on PATH? Set LEAN_BIN_PATH in .env.",
                        e, self.lean_bin
                    )),
                    contains_sorry: false,
                })
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Lean 把诊断信息写到 stderr
        let combined = if stderr.is_empty() {
            stdout.clone()
        } else {
            stderr.clone()
        };
        let contains_sorry = code.contains("sorry") || code.contains("admit");
        let success = output.status.success() && stderr.is_empty();

        Ok(LeanResult {
            success,
            output: if success {
                "no errors".to_string()
            } else {
                combined.clone()
            },
            error: if success { None } else { Some(combined) },
            contains_sorry,
        })
    }

    pub async fn check_file(&self, path: &PathBuf) -> Result<LeanResult> {
        let code = tokio::fs::read_to_string(path).await?;
        self.run(&code).await
    }
}
