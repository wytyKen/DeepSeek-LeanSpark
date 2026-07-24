import CodeMirror from '@uiw/react-codemirror';

interface Props {
  value: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
}

export function CodeEditor({ value, onChange, readOnly }: Props) {
  // Phase 1：npm 上暂无稳定的 Lean4 CodeMirror 语言包，
  // 这里先用纯编辑器（无语法高亮）。Phase 2 可改用
  // codemirror-lang-lean 或 leanc-client 的 LSP 集成。
  return (
    <CodeMirror
      value={value}
      height="auto"
      theme="light"
      readOnly={readOnly}
      extensions={[]}
      onChange={onChange}
      basicSetup={{ lineNumbers: true, highlightActiveLine: !readOnly }}
    />
  );
}
