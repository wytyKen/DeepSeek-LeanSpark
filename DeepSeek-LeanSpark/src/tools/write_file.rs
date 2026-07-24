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
