// 事件折叠组件：用于展示思考/工具调用/工具结果，默认折叠，三角形 ▶/▼ 切换。
import { useState } from 'react';
import type { AgentEvent } from '../../types';

interface Props {
  event: AgentEvent;
}

const LABEL: Record<AgentEvent['kind'], string> = {
  thinking: '思考',
  tool_call: '调用工具',
  tool_result: '工具结果',
  answer: '回答',
  error: '错误',
};

const COLOR: Record<AgentEvent['kind'], string> = {
  thinking: '#6b7280',
  tool_call: '#2563eb',
  tool_result: '#059669',
  answer: '#111827',
  error: '#dc2626',
};

export function EventCollapse({ event }: Props) {
  const [open, setOpen] = useState(false);
  const color = COLOR[event.kind];
  const label = LABEL[event.kind];

  // 摘要：tool_call 显示工具名 + 参数概要；tool_result 显示成功/失败摘要；thinking 显示前 80 字符
  const summary = makeSummary(event);

  return (
    <div className="event-collapse" style={{ margin: '2px 0' }}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="event-header"
        style={{
          border: 'none',
          background: 'transparent',
          cursor: 'pointer',
          padding: '4px 6px',
          color,
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
        <span style={{ fontWeight: 500 }}>{label}</span>
        {event.tool_name && (
          <span style={{ color: '#555' }}>: {event.tool_name}</span>
        )}
        {summary && (
          <span
            style={{
              color: '#888',
              fontSize: 11,
              marginLeft: 8,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              flex: 1,
            }}
          >
            {summary}
          </span>
        )}
      </button>
      {open && (
        <div
          className="event-body"
          style={{
            borderLeft: `3px solid ${color}`,
            margin: '4px 0 4px 14px',
            padding: '4px 10px',
            background: '#fafafa',
            fontSize: 12,
          }}
        >
          <pre
            style={{
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
              margin: 0,
              fontFamily: 'ui-monospace, Consolas, monospace',
              color: '#333',
              maxHeight: 400,
              overflow: 'auto',
            }}
          >
            {event.content.length > 4000
              ? event.content.slice(0, 4000) + '\n...(已截断)'
              : event.content}
          </pre>
        </div>
      )}
    </div>
  );
}

function makeSummary(event: AgentEvent): string {
  if (event.kind === 'tool_call') {
    const args = event.tool_args as Record<string, unknown> | null;
    if (!args) return '';
    // 取首个字符串字段的前 80 字符
    for (const v of Object.values(args)) {
      if (typeof v === 'string' && v.length > 0) {
        const s = v.replace(/\s+/g, ' ').trim();
        return s.length > 80 ? s.slice(0, 80) + '...' : s;
      }
    }
    return JSON.stringify(args).slice(0, 80);
  }
  if (event.kind === 'tool_result') {
    try {
      const obj = JSON.parse(event.content);
      if (obj.success === true) return '成功';
      if (obj.success === false) return `失败：${obj.error ?? ''}`.slice(0, 80);
    } catch {
      // 非 JSON
    }
    const s = event.content.replace(/\s+/g, ' ').trim();
    return s.length > 80 ? s.slice(0, 80) + '...' : s;
  }
  if (event.kind === 'thinking') {
    const s = event.content.replace(/\s+/g, ' ').trim();
    return s.length > 80 ? s.slice(0, 80) + '...' : s;
  }
  if (event.kind === 'error') {
    return event.content.slice(0, 80);
  }
  return '';
}
