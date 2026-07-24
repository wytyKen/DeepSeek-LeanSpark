// 助手消息：靠左无框，顶部显示 "LeanSpark 任务耗时 Xm XXs"，
// 下方是外层"思考过程"折叠（默认折叠），展开后内部各事件（思考/调用工具/工具结果）独立折叠，
// 分隔线后是回答正文（Markdown + KaTeX 渲染），底部显示文件变更标记。
import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeKatex from 'rehype-katex';
import type { AgentEvent, ChatMessage } from '../../types';
import { EventCollapse } from './EventCollapse';

interface Props {
  message: ChatMessage;
  onFormulaClick?: (latex: string) => void;
}

export function AssistantMessage({ message, onFormulaClick }: Props) {
  const duration = formatDuration(message.duration_ms);
  const events = (message.events ?? []).filter(
    (e) => e.kind !== 'answer' && e.kind !== 'error',
  );
  const changed = message.files_changed ?? [];
  const created = message.files_created ?? [];

  return (
    <div
      className="msg-row assistant-row"
      style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start' }}
    >
      {/* 顶部耗时行 */}
      {duration && (
        <div
          className="assistant-duration"
          style={{ color: '#888', fontSize: 12, marginBottom: 4 }}
        >
          LeanSpark 任务耗时 {duration}
        </div>
      )}

      {/* 外层"思考过程"折叠：包住所有 thinking/tool_call/tool_result */}
      {events.length > 0 && (
        <ThinkingProcessCollapse events={events} />
      )}

      {/* 分隔线 */}
      <hr
        style={{
          border: 'none',
          borderTop: '1px solid #eee',
          margin: '6px 0',
          width: '100%',
        }}
      />

      {/* 回答正文（Markdown + KaTeX） */}
      <div
        className="assistant-output markdown-body"
        style={{ width: '100%', fontSize: 14, lineHeight: 1.6 }}
      >
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          rehypePlugins={[[rehypeKatex, { throwOnError: false, strict: false }]]}
          components={{
            // 块级公式点击放大
            div: ({ children, ...rest }) => {
              // react-markdown 把 $$...$$ 包成 <div class="math math-display">
              const cls = (rest as { className?: string }).className ?? '';
              if (cls.includes('math-display')) {
                // 从 children 提取 LaTeX 文本
                const latex = extractText(children);
                return (
                  <div
                    {...rest}
                    style={{
                      position: 'relative',
                      cursor: 'pointer',
                      padding: '4px 0',
                    }}
                    onClick={() => onFormulaClick?.(latex)}
                    title="点击放大"
                  >
                    {children}
                  </div>
                );
              }
              return <div {...rest}>{children}</div>;
            },
          }}
        >
          {message.content}
        </ReactMarkdown>
      </div>

      {/* 文件变更标记：图标 + 文字，更接近 Trae Work 风格 */}
      {(changed.length > 0 || created.length > 0) && (
        <div
          className="assistant-files"
          style={{
            color: '#888',
            fontSize: 12,
            marginTop: 8,
            width: '100%',
            display: 'flex',
            gap: 16,
            flexWrap: 'wrap',
          }}
        >
          {changed.length > 0 && (
            <span title={changed.join(', ')}>
              <span aria-hidden style={{ marginRight: 4 }}>✎</span>
              {changed.length} 个文件已更改
            </span>
          )}
          {created.length > 0 && (
            <span title={created.join(', ')}>
              <span aria-hidden style={{ marginRight: 4 }}>📄</span>
              {created.length} 个文件已生成
            </span>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * 外层"思考过程"折叠：包住所有 thinking/tool_call/tool_result 事件。
 * 默认折叠，展开后内部各事件再独立折叠（由 EventCollapse 负责）。
 * 标题显示"思考过程 · N 步"，并按事件类型给出小计数。
 */
function ThinkingProcessCollapse({ events }: { events: AgentEvent[] }) {
  const [open, setOpen] = useState(false);

  const counts = countByKind(events);
  const summaryParts: string[] = [];
  if (counts.thinking > 0) summaryParts.push(`${counts.thinking} 思考`);
  if (counts.tool_call > 0) summaryParts.push(`${counts.tool_call} 调用`);
  if (counts.tool_result > 0) summaryParts.push(`${counts.tool_result} 结果`);
  const summary = summaryParts.join(' · ');

  return (
    <div className="thinking-process" style={{ width: '100%', marginBottom: 4 }}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="thinking-process-header"
        style={{
          border: 'none',
          background: 'transparent',
          cursor: 'pointer',
          padding: '4px 6px',
          color: '#6b7280',
          fontSize: 12,
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          textAlign: 'left',
          width: '100%',
        }}
      >
        <span style={{ display: 'inline-block', width: 10, textAlign: 'center' }}>
          {open ? '▼' : '▶'}
        </span>
        <span style={{ fontWeight: 500 }}>思考过程</span>
        <span style={{ color: '#9ca3af', fontSize: 11 }}>{summary}</span>
      </button>
      {open && (
        <div
          className="thinking-process-body"
          style={{
            borderLeft: '3px solid #e5e7eb',
            margin: '4px 0 4px 14px',
            padding: '2px 8px',
            background: '#fafafa',
          }}
        >
          {events.map((ev, i) => (
            <EventCollapse key={i} event={ev} />
          ))}
        </div>
      )}
    </div>
  );
}

function countByKind(events: AgentEvent[]): {
  thinking: number;
  tool_call: number;
  tool_result: number;
} {
  let thinking = 0;
  let tool_call = 0;
  let tool_result = 0;
  for (const e of events) {
    if (e.kind === 'thinking') thinking += 1;
    else if (e.kind === 'tool_call') tool_call += 1;
    else if (e.kind === 'tool_result') tool_result += 1;
  }
  return { thinking, tool_call, tool_result };
}

function formatDuration(ms?: number): string {
  if (!ms || ms <= 0) return '';
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  if (m === 0) return `${s}s`;
  return `${m}m ${String(s).padStart(2, '0')}s`;
}

// 从 React children 中提取纯文本（用于块级公式点击放大时拿 LaTeX 源码）
function extractText(children: unknown): string {
  if (typeof children === 'string') return children;
  if (Array.isArray(children)) return children.map(extractText).join('');
  if (children && typeof children === 'object') {
    const props = (children as { props?: { children?: unknown } }).props;
    if (props?.children) return extractText(props.children);
  }
  return '';
}
