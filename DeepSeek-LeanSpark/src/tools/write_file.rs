// write_file 工具：写入工作区内相对路径文件（创建或覆盖）
//
// 调用结果中带 `__files_changed` / `__files_created` 元字段，
// AgentLoop 据此填充 AgentEvent.files_changed / files_created。
use crate::tools::Tool;
use crate::workspace::WorkspaceManager;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct WriteFileTool {
    workspace: Arc<WorkspaceManager>,
}

impl WriteFileTool {
    pub fn new(workspace: Arc<WorkspaceManager>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn spec(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "写入当前工作区内指定相对路径的文件。若文件已存在则覆盖，不存在则创建（含父目录）。仅当用户已打开工作区时可用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "工作区内相对路径（POSIX 风格，如 'proofs/foo.lean'）。禁止绝对路径或 '..'。"
                        },
                        "content": {
                            "type": "string",
                            "description": "完整的文件内容（全量写入，非追加）。"
                        }
                    },
                    "required": ["path", "content"],
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
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing content argument"))?;

        match self.workspace.write_file(path, content).await {
            Ok(result) => {
                // 在结果里塞两个下划线开头的元字段，供 AgentLoop 解析
                let resp = json!({
                    "success": true,
                    "path": result.path,
                    "created": result.created,
                    "bytes": result.bytes,
                    "__files_changed": [result.path.clone()],
                    "__files_created": if result.created { vec![result.path.clone()] } else { vec![] }
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

    /// 构造测试用 WriteFileTool，工作区打开到临时目录。
    async fn make_tool() -> (WriteFileTool, Arc<WorkspaceManager>, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = Arc::new(WorkspaceManager::new());
        let path = tmp.path().to_str().unwrap().to_string();
        workspace.open(&path).await.unwrap();
        let tool = WriteFileTool::new(workspace.clone());
        (tool, workspace, tmp)
    }

    async fn call_tool(tool: &WriteFileTool, args: &Value) -> Value {
        let s = tool.call(args).await.unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn spec_has_correct_name_and_parameters() {
        let ws = Arc::new(WorkspaceManager::new());
        let tool = WriteFileTool::new(ws);
        let spec = tool.spec();
        assert_eq!(spec["function"]["name"], "write_file");
        let required = spec["function"]["parameters"]["required"]
            .as_array()
            .unwrap();
        let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(required_names.contains(&"path"), "required 应包含 path");
        assert!(
            required_names.contains(&"content"),
            "required 应包含 content"
        );
    }

    #[tokio::test]
    async fn creates_new_file_with_correct_metadata() {
        let (tool, ws, _tmp) = make_tool().await;
        let args = json!({ "path": "new.lean", "content": "theorem t : True := by trivial" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], true);
        assert_eq!(v["path"], "new.lean");
        assert_eq!(v["created"], true, "新文件应 created=true");
        assert_eq!(
            v["bytes"], 30,
            "\"theorem t : True := by trivial\" 长度为 30"
        );

        // __files_changed 元字段
        let changed = v["__files_changed"].as_array().unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], "new.lean");

        // __files_created 元字段
        let created = v["__files_created"].as_array().unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0], "new.lean");

        // 文件确实被写入
        let (content, _) = ws.read_file("new.lean").await.unwrap();
        assert_eq!(content, "theorem t : True := by trivial");
    }

    #[tokio::test]
    async fn overwrites_existing_file_sets_created_false() {
        let (tool, ws, _tmp) = make_tool().await;
        // 先写一次
        ws.write_file("existing.lean", "old content").await.unwrap();

        // 工具再覆盖
        let args = json!({ "path": "existing.lean", "content": "new content" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], true);
        assert_eq!(v["created"], false, "覆盖已有文件应 created=false");
        assert_eq!(v["bytes"], 11);

        // __files_created 应为空数组
        let created = v["__files_created"].as_array().unwrap();
        assert_eq!(created.len(), 0, "覆盖时 __files_created 应为空");

        // __files_changed 仍应包含该路径
        let changed = v["__files_changed"].as_array().unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], "existing.lean");

        // 内容确实被覆盖
        let (content, _) = ws.read_file("existing.lean").await.unwrap();
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn creates_parent_directories_automatically() {
        let (tool, ws, _tmp) = make_tool().await;
        let args = json!({
            "path": "proofs/chapter1/lemma1.lean",
            "content": "lemma l : True := by trivial"
        });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], true, "应自动创建父目录链");
        assert_eq!(v["created"], true);

        // 嵌套文件确实存在
        let (content, _) = ws.read_file("proofs/chapter1/lemma1.lean").await.unwrap();
        assert_eq!(content, "lemma l : True := by trivial");
    }

    #[tokio::test]
    async fn path_with_parent_dir_is_rejected() {
        let (tool, _ws, _tmp) = make_tool().await;
        let args = json!({ "path": "../escape.lean", "content": "x" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], false, "包含 .. 的路径应被拒绝");
        assert!(
            v["error"].as_str().unwrap().contains("超出")
                || v["error"].as_str().unwrap().contains("工作区"),
            "错误信息应提及工作区越界，实际: {}",
            v["error"]
        );
        // 不应有 __files_changed 元字段
        assert!(
            v.get("__files_changed").is_none(),
            "失败时不应填充 __files_changed"
        );
    }

    #[tokio::test]
    async fn missing_path_argument_returns_error() {
        let (tool, _ws, _tmp) = make_tool().await;
        let args = json!({ "content": "x" });
        let result = tool.call(&args).await;
        assert!(result.is_err(), "缺少 path 应返回 Err");
        assert!(
            result.unwrap_err().to_string().contains("path"),
            "错误信息应提及 path"
        );
    }

    #[tokio::test]
    async fn missing_content_argument_returns_error() {
        let (tool, _ws, _tmp) = make_tool().await;
        let args = json!({ "path": "foo.lean" });
        let result = tool.call(&args).await;
        assert!(result.is_err(), "缺少 content 应返回 Err");
        assert!(
            result.unwrap_err().to_string().contains("content"),
            "错误信息应提及 content"
        );
    }

    #[tokio::test]
    async fn workspace_not_open_returns_success_false() {
        let workspace = Arc::new(WorkspaceManager::new());
        let tool = WriteFileTool::new(workspace);
        let args = json!({ "path": "any.lean", "content": "x" });
        let v = call_tool(&tool, &args).await;

        assert_eq!(v["success"], false, "工作区未打开应返回 success=false");
        assert!(
            v["error"].as_str().unwrap().contains("工作区"),
            "错误信息应提及工作区未打开，实际: {}",
            v["error"]
        );
    }
}
