// 工作区管理器：跟踪当前打开的工作区根目录，提供文件树与读写能力。
use crate::workspace::paths::{ensure_within, relativize};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// 文件树深度上限（防止过深递归拖慢前端）
const MAX_DEPTH: usize = 5;
/// 单目录文件数上限
const MAX_ENTRIES_PER_DIR: usize = 1000;
/// 排除的目录名
const EXCLUDED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".lake",
    ".git",
    "dist",
    "build",
    ".vscode",
    ".idea",
];

#[derive(Clone)]
pub struct WorkspaceManager {
    root: Arc<RwLock<Option<PathBuf>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    pub path: String, // 相对工作区根的 POSIX 路径
    pub kind: String, // "file" | "dir"
    pub size: Option<u64>,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResult {
    pub path: String,
    pub created: bool, // true=新文件，false=覆盖
    pub bytes: usize,
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self {
            root: Arc::new(RwLock::new(None)),
        }
    }

    /// 打开工作区：校验路径存在且是目录，记录规范化后的绝对路径。
    pub async fn open(&self, path: &str) -> Result<PathBuf> {
        let p = PathBuf::from(path);
        if !p.exists() {
            return Err(anyhow!("路径不存在: {}", path));
        }
        let canonical = p.canonicalize()?;
        if !canonical.is_dir() {
            return Err(anyhow!("路径不是目录: {}", path));
        }
        let mut guard = self.root.write().await;
        *guard = Some(canonical.clone());
        Ok(canonical)
    }

    pub async fn close(&self) {
        let mut guard = self.root.write().await;
        *guard = None;
    }

    pub async fn current(&self) -> Option<PathBuf> {
        self.root.read().await.clone()
    }

    /// 列出工作区文件树。返回 None 表示未打开工作区。
    pub async fn list_tree(&self) -> Result<Option<FileNode>> {
        let root = self.root.read().await.clone();
        let Some(root) = root else {
            return Ok(None);
        };
        let node = Box::pin(walk(&root, &root, 0)).await?;
        Ok(Some(node))
    }

    /// 读取工作区内相对路径文件。
    pub async fn read_file(&self, rel_path: &str) -> Result<(String, PathBuf)> {
        let root = self.root.read().await.clone();
        let Some(root) = root else {
            return Err(anyhow!("工作区未打开"));
        };
        let abs = ensure_within(&root, Path::new(rel_path))?;
        let content = fs::read_to_string(&abs).await?;
        Ok((content, abs))
    }

    /// 写入工作区内相对路径文件（创建或覆盖）。
    pub async fn write_file(&self, rel_path: &str, content: &str) -> Result<WriteResult> {
        let root = self.root.read().await.clone();
        let Some(root) = root else {
            return Err(anyhow!("工作区未打开"));
        };
        let abs = ensure_within(&root, Path::new(rel_path))?;
        // 先创建父目录链
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).await?;
        }
        let created = !abs.exists();
        let bytes = content.len();
        fs::write(&abs, content).await?;
        let rel = relativize(&root, &abs).unwrap_or_else(|_| rel_path.to_string());
        Ok(WriteResult {
            path: rel,
            created,
            bytes,
        })
    }
}

/// 独立的递归函数（async 递归需 Box::pin）
async fn walk(abs: &Path, root: &Path, depth: usize) -> Result<FileNode> {
    let name = abs
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    let rel = relativize(root, abs).unwrap_or_else(|_| name.clone());

    let metadata = fs::metadata(abs).await?;
    if metadata.is_file() {
        return Ok(FileNode {
            name,
            path: rel,
            kind: "file".to_string(),
            size: Some(metadata.len()),
            children: None,
        });
    }

    // 目录：递归子项
    let mut children = Vec::new();
    if depth < MAX_DEPTH {
        let mut entries = fs::read_dir(abs).await?;
        let mut count = 0;
        while let Some(entry) = entries.next_entry().await? {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            // 排除隐藏文件与排除目录
            if entry_name.starts_with('.') {
                continue;
            }
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false)
                && EXCLUDED_DIRS.contains(&entry_name.as_str())
            {
                continue;
            }
            count += 1;
            if count > MAX_ENTRIES_PER_DIR {
                break;
            }
            let child = Box::pin(walk(&entry.path(), root, depth + 1)).await?;
            children.push(child);
        }
        // 目录排在前面，文件在后
        children.sort_by(|a, b| match (a.kind.as_str(), b.kind.as_str()) {
            ("dir", "file") => std::cmp::Ordering::Less,
            ("file", "dir") => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
    }

    Ok(FileNode {
        name,
        path: rel,
        kind: "dir".to_string(),
        size: None,
        children: Some(children),
    })
}
