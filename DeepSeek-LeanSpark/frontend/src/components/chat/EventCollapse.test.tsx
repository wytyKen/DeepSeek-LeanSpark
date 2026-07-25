// EventCollapse 组件测试
// 验证：默认折叠、点击展开、各 kind 标签/颜色、摘要生成、长内容截断。
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { EventCollapse } from './EventCollapse';
import type { AgentEvent } from '../../types';

describe('EventCollapse', () => {
  it('renders collapsed by default (content hidden)', () => {
    const event: AgentEvent = { kind: 'thinking', content: '深思考中...' };
    const { container } = render(<EventCollapse event={event} />);
    expect(screen.getByText('思考')).toBeInTheDocument();
    // 折叠态：event-body 不应渲染（摘要 span 可能含同文本，故用 .event-body 选择器）
    expect(container.querySelector('.event-body')).toBeNull();
  });

  it('expands on click and shows content', () => {
    const event: AgentEvent = { kind: 'thinking', content: '深思考中...' };
    const { container } = render(<EventCollapse event={event} />);
    fireEvent.click(screen.getByRole('button'));
    const body = container.querySelector('.event-body') as HTMLElement;
    expect(body).toBeTruthy();
    expect(body.querySelector('pre')?.textContent).toContain('深思考中...');
  });

  it('collapses on second click', () => {
    const event: AgentEvent = { kind: 'thinking', content: '内容' };
    const { container } = render(<EventCollapse event={event} />);
    const btn = screen.getByRole('button');
    fireEvent.click(btn); // 展开
    expect(container.querySelector('.event-body')).toBeTruthy();
    fireEvent.click(btn); // 折叠
    expect(container.querySelector('.event-body')).toBeNull();
  });

  it('shows tool_name for tool_call events', () => {
    const event: AgentEvent = {
      kind: 'tool_call',
      content: '',
      tool_name: 'run_lean_check',
      tool_args: { lean_code: 'theorem t : True := by trivial' },
    };
    render(<EventCollapse event={event} />);
    expect(screen.getByText('调用工具')).toBeInTheDocument();
    expect(screen.getByText(/run_lean_check/)).toBeInTheDocument();
  });

  it('shows "成功" summary for successful tool_result', () => {
    const event: AgentEvent = {
      kind: 'tool_result',
      content: JSON.stringify({ success: true, output: 'ok' }),
    };
    render(<EventCollapse event={event} />);
    expect(screen.getByText('成功')).toBeInTheDocument();
  });

  it('shows failure summary for failed tool_result', () => {
    const event: AgentEvent = {
      kind: 'tool_result',
      content: JSON.stringify({ success: false, error: 'syntax error' }),
    };
    render(<EventCollapse event={event} />);
    expect(screen.getByText(/失败/)).toBeInTheDocument();
    expect(screen.getByText(/syntax error/)).toBeInTheDocument();
  });

  it('truncates thinking content over 80 chars in summary', () => {
    const long = 'x'.repeat(120);
    const event: AgentEvent = { kind: 'thinking', content: long };
    render(<EventCollapse event={event} />);
    // 摘要应被截断为 80 字符 + "..."
    expect(screen.getByText(/x{80}\.\.\./)).toBeInTheDocument();
  });

  it('truncates displayed content over 4000 chars with notice', () => {
    const long = 'a'.repeat(5000);
    const event: AgentEvent = { kind: 'thinking', content: long };
    render(<EventCollapse event={event} />);
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByText(/\.\.\.\(已截断\)/)).toBeInTheDocument();
  });

  it('handles non-JSON tool_result content gracefully', () => {
    const event: AgentEvent = {
      kind: 'tool_result',
      content: 'plain text result',
    };
    render(<EventCollapse event={event} />);
    // 不应崩溃，摘要应为纯文本
    expect(screen.getByText('工具结果')).toBeInTheDocument();
  });

  it('uses correct label for each event kind', () => {
    const kinds: AgentEvent['kind'][] = ['thinking', 'tool_call', 'tool_result', 'answer', 'error'];
    const labels = ['思考', '调用工具', '工具结果', '回答', '错误'];
    for (let i = 0; i < kinds.length; i++) {
      const { unmount } = render(<EventCollapse event={{ kind: kinds[i], content: 'x' }} />);
      expect(screen.getByText(labels[i])).toBeInTheDocument();
      unmount();
    }
  });

  it('renders arrow indicator ▶ when collapsed, ▼ when expanded', () => {
    const event: AgentEvent = { kind: 'thinking', content: 'x' };
    render(<EventCollapse event={event} />);
    const btn = screen.getByRole('button');
    expect(btn.textContent).toContain('▶');
    fireEvent.click(btn);
    expect(btn.textContent).toContain('▼');
  });
});
