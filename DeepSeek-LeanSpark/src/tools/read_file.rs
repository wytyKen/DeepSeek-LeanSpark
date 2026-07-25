// read_file 工具：读取工作区内相对路径文件
use crate::tools::Tool;
use crate::workspace::WorkspaceManager;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct ReadFileTool {
    workspace: Arc<WorkspaceManager>,
}

impl ReadFileTool {
    pub fn new(workspace: Arc<WorkspaceManager>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "读取当前工作区内指定相对路径的文件内容。仅当用户已打开工作区时可用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "工作区内相对路径（POSIX 风格，如 'src/foo.lean'）。禁止使用绝对路径或包含 '..'。"
                        }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                },
                "strict": true
            }
        })
    }

    async fn call(&self, args: &Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing path argument"))?;

        match self.workspace.read_file(path).await {
            Ok((content, _abs)) => {
                let resp = json!({
                    "success": true,
                    "path": path,
                    "content": content,
                    "bytes": content.len()
                });
                Ok(serde_json::to_string(&resp)?)
            }
            Err(e) => {
                let resp = json!({
                    "success": false,
                    "path": path,
                    "error": e.to_string()
                });
                Ok(serde_json::to_string(&resp)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::WorkspaceManager;
    use serde_json::Value;
    use std::sync::Arc;

    /// 构造测试用 ReadFileTool，工作区打开到临时目录。
    /// 返回 (tool, workspace, tmp)：调用方需持有 _tmp 防止目录被回收。
    async fn make_tool() -> (ReadFileTool, Arc<WorkspaceManager>, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = Arc::new(WorkspaceManager::new());
        let path = tmp.path().to_str().unwrap().to_string();
        workspace.open(&path).await.unwrap();
        let tool = ReadFileTool::new(workspace.clone());
        (tool, workspace, tmp)
    }

    async fn call_tool(tool: &ReadFileTool, args: &Value) -> Value {
        let s = tool.call(args).await.unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn spec_has_correct_name_and_parameters() {
        let ws = Arc::new(WorkspaceManager::new());
        let tool = ReadFileTool::new(ws);
        let spec = tool.spec();
        assert_eq!(spec["function"]["name"], "read_file");
        assert_eq!(spec["function"]["parameters"]["required"][0], "path");
        assert_eq!(
            spec["function"]["parameters"]["properties"]["path"]["type"],
            "string"
        );
    }

    #[tokio::test]
    async fn reads_existing_file_successfully() {
        let (tool, ws, _tmp) = make_tool().await;
        // 预先写入文件
        ws.write_file("hello.lean", "theorem t : True := by trivial")
            .await
            .unwrap();

        let args = json!({ "path": "hello.lean" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], true);
        assert_eq!(v["path"], "hello.lean");
        assert_eq!(v["content"], "theorem t : True := by trivial");
        assert_eq!(
            v["bytes"], 30,
            "\"theorem t : True := by trivial\" 长度为 30"
        );
    }

    #[tokio::test]
    async fn reads_file_in_subdirectory() {
        let (tool, ws, _tmp) = make_tool().await;
        ws.write_file("src/foo.lean", "by rfl").await.unwrap();

        let args = json!({ "path": "src/foo.lean" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], true);
        assert_eq!(v["content"], "by rfl");
    }

    #[tokio::test]
    async fn nonexistent_file_returns_success_false_with_error() {
        let (tool, _ws, _tmp) = make_tool().await;
        let args = json!({ "path": "does_not_exist.lean" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], false);
        assert_eq!(v["path"], "does_not_exist.lean");
        assert!(!v["error"].as_str().unwrap().is_empty(), "应返回非空错误信息");
    }

    #[tokio::test]
    async fn path_with_parent_dir_is_rejected() {
        // `..` 应被 ensure_within 拒绝（路径越界）
        let (tool, _ws, _tmp) = make_tool().await;
        let args = json!({ "path": "../escape.lean" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], false, "包含 .. 的路径应被拒绝");
        assert!(
            v["error"].as_str().unwrap().contains("工作区")
                || v["error"].as_str().unwrap().contains("超出"),
            "错误信息应提及工作区越界，实际: {}",
            v["error"]
        );
    }

    #[tokio::test]
    async fn missing_path_argument_returns_error() {
        let (tool, _ws, _tmp) = make_tool().await;
        let args = json!({});
        let result = tool.call(&args).await;
        assert!(result.is_err(), "缺少 path 参数应返回 Err");
        assert!(
            result.unwrap_err().to_string().contains("path"),
            "错误信息应提及 path"
        );
    }

    #[tokio::test]
    async fn workspace_not_open_returns_success_false() {
        // 不打开工作区
        let workspace = Arc::new(WorkspaceManager::new());
        let tool = ReadFileTool::new(workspace);
        let args = json!({ "path": "any.lean" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], false, "工作区未打开应返回 success=false");
        assert!(
            v["error"].as_str().unwrap().contains("工作区"),
            "错误信息应提及工作区未打开，实际: {}",
            v["error"]
        );
    }
}
