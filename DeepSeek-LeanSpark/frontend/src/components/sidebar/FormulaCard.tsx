// 单个公式卡片：KaTeX 渲染 + 复制源码按钮 + 点击放大。
import { useEffect, useState } from 'react';
import { CopyButton } from '../common/CopyButton';

interface Props {
  latex: string;
  source?: string;
  onClick?: (latex: string) => void;
}

export function FormulaCard({ latex, source, onClick }: Props) {
  const [html, setHtml] = useState<string>('');

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const katex = await import('katex');
        const rendered = katex.renderToString(latex, {
          displayMode: true,
          throwOnError: false,
          strict: false,
        });
        if (!cancelled) setHtml(rendered);
      } catch {
        if (!cancelled) setHtml(`<code>${escapeHtml(latex)}</code>`);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [latex]);

  return (
    <div
      className="formula-card"
      style={{
        border: '1px solid #e5e5e5',
        borderRadius: 6,
        padding: 10,
        background: '#fff',
        marginBottom: 8,
        cursor: onClick ? 'pointer' : 'default',
      }}
      onClick={() => onClick?.(latex)}
    >
      <div
        className="formula-rendered"
        style={{ textAlign: 'center', padding: '4px 0', fontSize: 16 }}
        dangerouslySetInnerHTML={{ __html: html }}
      />
      {source && (
        <div
          style={{
            marginTop: 6,
            paddingTop: 6,
            borderTop: '1px dashed #eee',
            fontSize: 11,
            color: '#888',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>
            {source}
          </span>
          <div onClick={(e) => e.stopPropagation()}>
            <CopyButton text={latex} label="复制 LaTeX" />
          </div>
        </div>
      )}
      {!source && (
        <div
          style={{
            marginTop: 6,
            display: 'flex',
            justifyContent: 'flex-end',
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <CopyButton text={latex} label="复制 LaTeX" />
        </div>
      )}
    </div>
  );
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
