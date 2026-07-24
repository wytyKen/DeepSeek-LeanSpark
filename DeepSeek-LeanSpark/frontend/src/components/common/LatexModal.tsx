// LaTeX 公式放大 Modal：用 KaTeX 渲染大号公式，含关闭按钮与复制源码按钮。
import { useEffect, useState } from 'react';
import { CopyButton } from './CopyButton';

interface Props {
  latex: string;
  onClose: () => void;
}

export function LatexModal({ latex, onClose }: Props) {
  const [html, setHtml] = useState<string>('');

  // Esc 关闭
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  // 异步加载 KaTeX 并渲染
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
      className="latex-modal-overlay"
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.45)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
    >
      <div
        className="latex-modal"
        onClick={(e) => e.stopPropagation()}
        style={{
          background: '#fff',
          borderRadius: 8,
          padding: '24px 32px',
          maxWidth: '80vw',
          maxHeight: '80vh',
          overflow: 'auto',
          boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
          position: 'relative',
        }}
      >
        <button
          type="button"
          onClick={onClose}
          aria-label="关闭"
          style={{
            position: 'absolute',
            top: 8,
            right: 8,
            border: 'none',
            background: 'transparent',
            fontSize: 20,
            cursor: 'pointer',
            color: '#888',
          }}
        >
          ×
        </button>
        <div
          className="latex-modal-content"
          style={{ fontSize: 24, lineHeight: 1.6, textAlign: 'center', margin: '12px 0' }}
          dangerouslySetInnerHTML={{ __html: html }}
        />
        <div
          style={{
            marginTop: 16,
            paddingTop: 12,
            borderTop: '1px solid #eee',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            gap: 12,
          }}
        >
          <code
            style={{
              fontSize: 12,
              color: '#666',
              fontFamily: 'monospace',
              wordBreak: 'break-all',
              flex: 1,
            }}
          >
            {latex}
          </code>
          <CopyButton text={latex} label="复制 LaTeX" size="md" />
        </div>
      </div>
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
