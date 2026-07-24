// 代码编辑器：用于在工作区中查看/编辑 .lean / .md 等文本文件。
// 用 @uiw/react-codemirror，无语法高亮（npm 上暂无稳定的 Lean4 CodeMirror 语言包）。
// Phase 2 可改用 codemirror-lang-lean 或 leanc-client 的 LSP 集成。
import CodeMirror from '@uiw/react-codemirror';

interface Props {
  /** 当前打开文件的相对路径，用于显示标题；为空则不显示头部 */
  filePath?: string | null;
  value: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  onSave?: () => void;
}

export function CodeEditor({ filePath, value, onChange, readOnly, onSave }: Props) {
  return (
    <div
      className="code-editor"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        border: '1px solid #e5e5e5',
        borderRadius: 4,
        overflow: 'hidden',
      }}
    >
      {filePath && (
        <div
          className="code-editor-header"
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '4px 10px',
            background: '#f8f8f8',
            borderBottom: '1px solid #e5e5e5',
            fontSize: 12,
          }}
        >
          <code
            style={{
              fontFamily: 'ui-monospace, Consolas, monospace',
              color: '#333',
            }}
          >
            {filePath}
          </code>
          {onSave && !readOnly && (
            <button
              type="button"
              onClick={onSave}
              style={{
                padding: '2px 10px',
                fontSize: 11,
                background: '#2563eb',
                color: '#fff',
                border: 'none',
                borderRadius: 3,
                cursor: 'pointer',
              }}
            >
              保存
            </button>
          )}
        </div>
      )}
      <div style={{ flex: 1, overflow: 'auto' }}>
        <CodeMirror
          value={value}
          height="auto"
          theme="light"
          readOnly={readOnly}
          extensions={[]}
          onChange={onChange}
          basicSetup={{ lineNumbers: true, highlightActiveLine: !readOnly }}
        />
      </div>
    </div>
  );
}
