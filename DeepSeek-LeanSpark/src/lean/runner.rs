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
                    // 即使 lean 二进制不存在，也要基于 code 检测 sorry/admit，
                    // 否则含 sorry 的代码在 lean 未安装时会逃过安全检查。
                    contains_sorry: code.contains("sorry") || code.contains("admit"),
                });
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;

    /// 全局锁：强制所有 lean runner 测试串行执行。
    /// 原因：每个 run() 调用都会在系统临时目录创建 leanspark_*.lean 文件，
    /// `temp_file_cleaned_up_after_run` 测试会扫描临时目录，若与其他测试并行会误判。
    static TEST_LOCK: Mutex<()> = Mutex::const_new(());

    /// 用不存在的 lean 二进制构造 runner。run() 会进入 IO 错误分支，
    /// 返回 success=false 的 LeanResult（不 panic），同时 still 设置 contains_sorry。
    fn make_broken_runner() -> LeanRunner {
        LeanRunner::new("/nonexistent/lean_binary_xyz".to_string())
    }

    #[tokio::test]
    async fn detects_sorry_in_code() {
        let _g = TEST_LOCK.lock().await;
        let runner = make_broken_runner();
        let r = runner.run("theorem t : True := by sorry").await.unwrap();
        assert!(r.contains_sorry, "代码含 sorry 应被检测到");
    }

    #[tokio::test]
    async fn detects_admit_in_code() {
        let _g = TEST_LOCK.lock().await;
        let runner = make_broken_runner();
        let r = runner.run("theorem t : True := by admit").await.unwrap();
        assert!(
            r.contains_sorry,
            "代码含 admit 应被检测到（contains_sorry=true）"
        );
    }

    #[tokio::test]
    async fn no_sorry_when_code_clean() {
        let _g = TEST_LOCK.lock().await;
        let runner = make_broken_runner();
        let r = runner.run("theorem t : True := by trivial").await.unwrap();
        assert!(!r.contains_sorry, "干净代码不应触发 sorry 检测");
    }

    #[tokio::test]
    async fn detects_sorry_in_comment_too() {
        let _g = TEST_LOCK.lock().await;
        // 当前实现是字符串包含匹配，注释里的 sorry 也会被检测（保守策略，可接受）
        let runner = make_broken_runner();
        let r = runner
            .run("-- TODO: remove sorry later\ntheorem t : True := by trivial")
            .await
            .unwrap();
        assert!(r.contains_sorry, "注释里的 sorry 也应被保守检测");
    }

    #[tokio::test]
    async fn nonexistent_lean_binary_returns_failure_with_io_error() {
        let _g = TEST_LOCK.lock().await;
        let runner = make_broken_runner();
        let r = runner.run("theorem t : True := by trivial").await.unwrap();
        assert!(!r.success, "lean 二进制不存在时 success 应为 false");
        assert!(
            r.error.is_some(),
            "error 字段应被填充（含 IO 错误信息），实际: {:?}",
            r.error
        );
        let err = r.error.as_ref().unwrap();
        assert!(
            err.contains("failed to execute lean"),
            "错误信息应含 'failed to execute lean'，实际: {}",
            err
        );
        assert!(
            err.contains("/nonexistent/lean_binary_xyz"),
            "错误信息应包含二进制路径，实际: {}",
            err
        );
    }

    #[tokio::test]
    async fn temp_file_cleaned_up_after_run() {
        let _g = TEST_LOCK.lock().await;
        // 验证 run() 完成后，临时目录中没有 leanspark_*.lean 残留。
        // 由于持有了 TEST_LOCK，不会有其他并行 lean 测试同时创建临时文件。
        let runner = make_broken_runner();
        let _ = runner.run("theorem t : True := by trivial").await.unwrap();

        let temp = std::env::temp_dir();
        let mut entries = tokio::fs::read_dir(&temp).await.unwrap();
        let mut leftover: Vec<String> = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("leanspark_") && name.ends_with(".lean") {
                leftover.push(name);
            }
        }
        assert!(
            leftover.is_empty(),
            "run() 后临时目录不应有 leanspark_*.lean 残留，发现: {:?}",
            leftover
        );
    }

    #[tokio::test]
    async fn empty_code_still_runs() {
        let _g = TEST_LOCK.lock().await;
        // 边界：空代码也应能进入 run()，不应 panic
        let runner = make_broken_runner();
        let r = runner.run("").await.unwrap();
        // lean 不存在 → success=false，但 contains_sorry 应为 false（空字符串不含 sorry）
        assert!(!r.success);
        assert!(!r.contains_sorry, "空代码不应被检测为含 sorry");
    }

    #[tokio::test]
    async fn check_file_reads_then_runs() {
        let _g = TEST_LOCK.lock().await;
        // check_file 应先读取文件内容，再调用 run
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.lean");
        tokio::fs::write(&file_path, "theorem t : True := by sorry")
            .await
            .unwrap();

        let runner = make_broken_runner();
        let r = runner.check_file(&file_path).await.unwrap();
        assert!(
            r.contains_sorry,
            "check_file 应读到 sorry 并设置 contains_sorry"
        );
    }

    #[tokio::test]
    async fn check_file_errors_on_nonexistent_file() {
        let _g = TEST_LOCK.lock().await;
        let runner = make_broken_runner();
        let result = runner
            .check_file(&PathBuf::from("/nonexistent/path/file.lean"))
            .await;
        assert!(result.is_err(), "读取不存在的文件应返回 Err");
    }
}
