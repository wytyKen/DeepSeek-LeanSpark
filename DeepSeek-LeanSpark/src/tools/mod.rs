use crate::lean::LeanRunner;
use crate::tools::lean_check::LeanCheckTool;
use crate::tools::proof_state::ProofStateTool;
use crate::tools::read_file::ReadFileTool;
use crate::tools::search::SearchMathlibTool;
use crate::tools::write_file::WriteFileTool;
use crate::workspace::WorkspaceManager;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub mod lean_check;
pub mod proof_state;
pub mod read_file;
pub mod search;
pub mod write_file;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn spec(&self) -> Value;
    async fn call(&self, args: &Value) -> Result<String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    /// 是否已注册工作区相关工具（read_file/write_file）
    workspace_tools_registered: bool,
}

impl ToolRegistry {
    pub fn new(lean: Arc<LeanRunner>) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        let lean_check = Arc::new(LeanCheckTool::new(lean));
        let search = Arc::new(SearchMathlibTool::new());
        let proof_state = Arc::new(ProofStateTool::new());
        tools.insert(lean_check.name().to_string(), lean_check);
        tools.insert(search.name().to_string(), search);
        tools.insert(proof_state.name().to_string(), proof_state);
        Self {
            tools,
            workspace_tools_registered: false,
        }
    }

    /// 启动时一次性构造：基础工具 + 工作区工具（工作区工具内部会检查是否已打开）。
    pub fn new_with_workspace(lean: Arc<LeanRunner>, workspace: Arc<WorkspaceManager>) -> Self {
        let mut reg = Self::new(lean);
        reg.register_workspace_tools(workspace);
        reg
    }

    /// 注册工作区工具（read_file / write_file）。
    /// 调用幂等：重复调用不会重复注册。
    pub fn register_workspace_tools(&mut self, workspace: Arc<WorkspaceManager>) {
        if self.workspace_tools_registered {
            return;
        }
        let read = Arc::new(ReadFileTool::new(workspace.clone()));
        let write = Arc::new(WriteFileTool::new(workspace));
        self.tools.insert(read.name().to_string(), read);
        self.tools.insert(write.name().to_string(), write);
        self.workspace_tools_registered = true;
    }

    pub fn specs(&self) -> Vec<Value> {
        self.tools.values().map(|t| t.spec()).collect()
    }

    pub async fn dispatch(&self, name: &str, args: &Value) -> Result<String> {
        match self.tools.get(name) {
            Some(tool) => tool.call(args).await,
            None => anyhow::bail!("unknown tool: {}", name),
        }
    }
}
