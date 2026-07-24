// 工作区切换器：顶部显示当前工作区路径，提供打开/关闭按钮。
// Tauri 桌面壳下使用原生文件对话框；Web 形态降级为 prompt 输入路径。
import type { UseWorkspaceResult } from '../../hooks/useWorkspace';
import { isTauri, pickDirectory } from '../../lib/tauri';

interface Props {
  workspace: UseWorkspaceResult;
}

export function WorkspaceSwitcher({ workspace }: Props) {
  const { open, path, loading, error } = workspace;

  const handleOpen = async () => {
    // Tauri 环境：原生文件对话框；Web 环境：prompt 降级（pickDirectory 内部处理）
    const selected = await pickDirectory(path ?? '');
    if (!selected) return;
    await workspace.openWorkspace(selected);
  };

  const handleClose = async () => {
    if (window.confirm('确定关闭当前工作区？')) {
      await workspace.closeWorkspace();
    }
  };

  return (
    <div
      className="workspace-switcher"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '4px 8px',
        fontSize: 12,
      }}
    >
      <span style={{ color: '#888' }}>工作区:</span>
      {open ? (
        <>
          <code
            title={path ?? ''}
            style={{
              color: '#2563eb',
              fontFamily: 'ui-monospace, Consolas, monospace',
              maxWidth: 280,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              display: 'inline-block',
              verticalAlign: 'middle',
            }}
          >
            {path}
          </code>
          <button
            type="button"
            onClick={handleClose}
            disabled={loading}
            style={{
              padding: '2px 8px',
              fontSize: 11,
              background: '#f5f5f5',
              border: '1px solid #e5e5e5',
              borderRadius: 4,
              cursor: 'pointer',
            }}
          >
            关闭
          </button>
        </>
      ) : (
        <button
          type="button"
          onClick={handleOpen}
          disabled={loading}
          style={{
            padding: '2px 10px',
            fontSize: 11,
            background: '#2563eb',
            color: '#fff',
            border: 'none',
            borderRadius: 4,
            cursor: 'pointer',
          }}
        >
          {loading ? '打开中...' : '打开文件夹'}
        </button>
      )}
      {error && (
        <span style={{ color: '#dc2626', fontSize: 11 }}>{error}</span>
      )}
      {!isTauri() && (
        <span style={{ color: '#9ca3af', fontSize: 10 }}>
          （Web 形态；Tauri 桌面壳下将使用原生文件对话框）
        </span>
      )}
    </div>
  );
}
