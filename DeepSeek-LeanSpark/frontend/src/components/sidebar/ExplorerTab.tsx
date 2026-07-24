// 资源管理器 Tab：顶部显示工作区名（文件夹名）+ 完整路径（工具提示）+ 刷新按钮，
// 下方文件树。未打开工作区时给出引导提示。
import type { UseWorkspaceResult } from '../../hooks/useWorkspace';
import { FileTree } from './FileTree';

interface Props {
  workspace: UseWorkspaceResult;
  selectedPath: string | null;
  onOpenFile: (relPath: string) => void;
}

export function ExplorerTab({ workspace, selectedPath, onOpenFile }: Props) {
  const { open, path, tree, refreshTree } = workspace;

  if (!open) {
    return (
      <div
        style={{
          padding: 16,
          color: '#888',
          fontSize: 13,
          textAlign: 'center',
        }}
      >
        <div style={{ marginBottom: 8 }}>未打开工作区</div>
        <div style={{ fontSize: 11, color: '#aaa' }}>
          点击顶部"打开文件夹"加载工作区
        </div>
      </div>
    );
  }

  // 工作区名：取路径最后一段作为标题（类似 VSCode 资源管理器）
  const folderName = path ? path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? path : '';
  // 工作区内的数学相关文件计数（用于在标题旁显示概要）
  const mathFileCount = tree ? countMathFiles(tree) : 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 8px',
          borderBottom: '1px solid #eee',
          background: '#fafafa',
        }}
      >
        <div
          style={{
            flex: 1,
            minWidth: 0,
            display: 'flex',
            flexDirection: 'column',
            gap: 1,
          }}
          title={path ?? ''}
        >
          <div
            style={{
              fontSize: 12,
              color: '#1f2937',
              fontWeight: 500,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            📁 {folderName}
          </div>
          <code
            style={{
              fontSize: 10,
              color: '#9ca3af',
              fontFamily: 'ui-monospace, Consolas, monospace',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {path}
          </code>
        </div>
        <button
          type="button"
          onClick={refreshTree}
          title="刷新文件树"
          style={{
            border: '1px solid #e5e5e5',
            background: '#fff',
            borderRadius: 3,
            padding: '2px 6px',
            fontSize: 11,
            cursor: 'pointer',
            flexShrink: 0,
          }}
        >
          ⟳
        </button>
      </div>
      {mathFileCount > 0 && (
        <div
          style={{
            padding: '2px 8px',
            fontSize: 10,
            color: '#9ca3af',
            background: '#fff',
            borderBottom: '1px solid #f3f4f6',
          }}
        >
          {mathFileCount} 个数学相关文件（.lean / .tex / .md / .pdf）
        </div>
      )}
      <div style={{ flex: 1, overflow: 'auto', padding: '4px 0' }}>
        <FileTree
          node={tree}
          selectedPath={selectedPath}
          onOpenFile={onOpenFile}
        />
      </div>
    </div>
  );
}

/** 统计工作区内数学相关文件数（.lean / .tex / .md / .pdf） */
function countMathFiles(node: { kind: string; name?: string; children?: unknown[] | null }): number {
  if (node.kind === 'file') {
    const n = node.name ?? '';
    if (
      n.endsWith('.lean') ||
      n.endsWith('.tex') ||
      n.endsWith('.md') ||
      n.endsWith('.pdf')
    ) {
      return 1;
    }
    return 0;
  }
  let count = 0;
  for (const child of node.children ?? []) {
    count += countMathFiles(child as never);
  }
  return count;
}
