// 文件树：递归展示工作区文件结构，目录优先排序。点击文件触发 onOpenFile。
// 不依赖 react-arborist（其虚拟化对小工作区过度复杂），用原生递归实现更简洁可靠。
import { useState } from 'react';
import type { FileNode } from '../../types';

interface Props {
  node: FileNode | null;
  /** 当前选中文件的相对路径（用于高亮） */
  selectedPath?: string | null;
  onOpenFile: (relPath: string) => void;
}

export function FileTree({ node, selectedPath, onOpenFile }: Props) {
  if (!node) {
    return <div style={{ padding: 8, color: '#999', fontSize: 12 }}>未打开工作区</div>;
  }
  return (
    <div className="file-tree" style={{ fontSize: 13, userSelect: 'none' }}>
      <TreeItem
        node={node}
        depth={0}
        selectedPath={selectedPath ?? null}
        onOpenFile={onOpenFile}
        isRoot
      />
    </div>
  );
}

interface TreeItemProps {
  node: FileNode;
  depth: number;
  selectedPath: string | null;
  onOpenFile: (relPath: string) => void;
  isRoot?: boolean;
}

function TreeItem({ node, depth, selectedPath, onOpenFile, isRoot }: TreeItemProps) {
  // 根节点不显示名称（已是工作区路径），直接展开子项
  if (isRoot) {
    if (node.kind !== 'dir' || !node.children) {
      return null;
    }
    return (
      <div>
        {node.children.map((child) => (
          <TreeItem
            key={child.path}
            node={child}
            depth={depth}
            selectedPath={selectedPath}
            onOpenFile={onOpenFile}
          />
        ))}
      </div>
    );
  }

  const indent = 12 + depth * 14;
  const isSelected = selectedPath === node.path;

  if (node.kind === 'file') {
    return (
      <div
        className={`tree-item file ${isSelected ? 'selected' : ''}`}
        onClick={() => onOpenFile(node.path)}
        style={{
          padding: `2px 8px 2px ${indent}px`,
          cursor: 'pointer',
          color: isSelected ? '#2563eb' : '#333',
          background: isSelected ? '#eff6ff' : 'transparent',
          fontWeight: isSelected ? 500 : 400,
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
        title={node.path}
      >
        <FileIcon name={node.name} />
        <span style={{ marginLeft: 4 }}>{node.name}</span>
      </div>
    );
  }

  // 目录：可折叠
  return (
    <FolderItem
      node={node}
      depth={depth}
      indent={indent}
      selectedPath={selectedPath}
      onOpenFile={onOpenFile}
    />
  );
}

function FolderItem({
  node,
  depth,
  indent,
  selectedPath,
  onOpenFile,
}: {
  node: FileNode;
  depth: number;
  indent: number;
  selectedPath: string | null;
  onOpenFile: (relPath: string) => void;
}) {
  const [open, setOpen] = useState(true);
  const children = node.children ?? [];

  return (
    <div>
      <div
        className="tree-item folder"
        onClick={() => setOpen((v) => !v)}
        style={{
          padding: `2px 8px 2px ${indent}px`,
          cursor: 'pointer',
          color: '#555',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
        title={node.path}
      >
        <span style={{ display: 'inline-block', width: 12, textAlign: 'center' }}>
          {open ? '▼' : '▶'}
        </span>
        <span style={{ marginLeft: 2 }}>📁</span>
        <span style={{ marginLeft: 4 }}>{node.name}</span>
      </div>
      {open &&
        children.map((child) => (
          <TreeItem
            key={child.path}
            node={child}
            depth={depth + 1}
            selectedPath={selectedPath}
            onOpenFile={onOpenFile}
          />
        ))}
    </div>
  );
}

function FileIcon({ name }: { name: string }) {
  if (name.endsWith('.lean')) return <span>📘</span>;
  if (name.endsWith('.md')) return <span>📝</span>;
  if (name.endsWith('.tex')) return <span>📄</span>;
  if (name.endsWith('.pdf')) return <span>📕</span>;
  if (name.endsWith('.json')) return <span>🔧</span>;
  if (name.endsWith('.toml')) return <span>🔧</span>;
  return <span>📄</span>;
}
