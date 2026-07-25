// 工作区路径安全校验：所有相对路径必须在根目录之内。
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// 校验 `target` 解析后仍在 `root` 之内，返回规范化的绝对路径。
///
/// - 若 `target` 不存在（例如写入新文件），先规范化其父目录再拼接文件名。
/// - 防御符号链接穿越、`..` 路径注入、绝对路径注入。
pub fn ensure_within(root: &Path, target: &Path) -> Result<PathBuf> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| anyhow!("无法规范化工作区根目录 {:?}: {}", root, e))?;

    // 处理两种输入：相对路径（相对 root）或绝对路径
    let abs_target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        canonical_root.join(target)
    };

    // 若目标已存在，直接 canonicalize 校验
    if abs_target.exists() {
        let canonical_target = abs_target
            .canonicalize()
            .map_err(|e| anyhow!("无法规范化目标路径 {:?}: {}", abs_target, e))?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(anyhow!("路径 {:?} 超出工作区根目录 {:?}", target, root));
        }
        return Ok(canonical_target);
    }

    // 目标不存在（写入新文件场景）：规范化父目录，再拼接文件名
    let parent = abs_target
        .parent()
        .ok_or_else(|| anyhow!("目标路径 {:?} 没有父目录", abs_target))?;
    if !parent.exists() {
        // 父目录也不存在——允许创建（write_file 会先创建目录链）
        // 但仍需校验父目录路径不会逃出 root
        return validate_nonexistent_path(&canonical_root, &abs_target);
    }
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| anyhow!("无法规范化父目录 {:?}: {}", parent, e))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(anyhow!("路径 {:?} 超出工作区根目录 {:?}", target, root));
    }
    let file_name = abs_target
        .file_name()
        .ok_or_else(|| anyhow!("目标路径 {:?} 没有文件名", abs_target))?;
    Ok(canonical_parent.join(file_name))
}

/// 对完全不存在的路径（父目录也不存在）做校验：
/// 通过逐段拼接并规范化 `..` 来判断是否在 root 内。
///
/// 注意：调用方传入的 `target` 通常是 `canonical_root.join(rel)` 后的绝对路径，
/// 在 Windows 上 canonicalize 会带上 `\\?\` UNC 前缀。如果直接遍历 components，
/// `Component::Prefix` 会被错误地当作"非法前缀"拒绝。因此这里先 strip_prefix
/// canonical_root 得到相对部分，再逐段处理。
fn validate_nonexistent_path(root: &Path, target: &Path) -> Result<PathBuf> {
    let canonical_root = root.canonicalize()?;
    // target 必须以 canonical_root 为前缀（调用方已 join），否则视为越界
    let rel = target
        .strip_prefix(&canonical_root)
        .map_err(|_| anyhow!("路径 {:?} 超出工作区根目录 {:?}", target, root))?;
    // 遍历相对部分的各段：遇 `..` 弹栈，遇 `.` 跳过，遇 Normal 压栈
    let mut acc = canonical_root.clone();
    for comp in rel.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !acc.pop() {
                    return Err(anyhow!("路径 {:?} 超出工作区根目录", target));
                }
            }
            Component::Normal(s) => acc.push(s),
            Component::Prefix(_) | Component::RootDir => {
                // strip_prefix 成功后不应再出现这些段，出现即视为非法
                return Err(anyhow!("路径 {:?} 包含非法前缀", target));
            }
        }
    }
    if !acc.starts_with(&canonical_root) {
        return Err(anyhow!("路径 {:?} 超出工作区根目录", target));
    }
    Ok(acc)
}

/// 把绝对路径转成相对工作区根的字符串（用于返回给前端/LLM）。
pub fn relativize(root: &Path, abs: &Path) -> Result<String> {
    let canonical_root = root.canonicalize()?;
    let canonical_abs = abs.canonicalize()?;
    let rel = canonical_abs
        .strip_prefix(&canonical_root)
        .map_err(|_| anyhow!("路径 {:?} 不在工作区内", abs))?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn within_root_ok() {
        let tmp = std::env::temp_dir().join(format!("ws_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir(&tmp).unwrap();
        let f = tmp.join("a.txt");
        fs::write(&f, "x").unwrap();
        let r = ensure_within(&tmp, Path::new("a.txt")).unwrap();
        assert_eq!(r, f.canonicalize().unwrap());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn escape_root_rejected() {
        let tmp = std::env::temp_dir().join(format!("ws_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir(&tmp).unwrap();
        let r = ensure_within(&tmp, Path::new("../escape.txt"));
        assert!(r.is_err());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn nonexistent_within_root_ok() {
        let tmp = std::env::temp_dir().join(format!("ws_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir(&tmp).unwrap();
        let r = ensure_within(&tmp, Path::new("new.txt")).unwrap();
        assert!(r.starts_with(tmp.canonicalize().unwrap()));
        fs::remove_dir_all(&tmp).unwrap();
    }
}
