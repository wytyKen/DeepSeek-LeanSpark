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
