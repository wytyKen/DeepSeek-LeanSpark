// UserBubble 组件测试
// 验证：靠右对齐、灰色背景、圆角、不显示"你"、纯文本渲染、内容转义。
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { UserBubble } from './UserBubble';

describe('UserBubble', () => {
  it('renders plain text content', () => {
    render(<UserBubble text="证明 1+2=3" />);
    expect(screen.getByText('证明 1+2=3')).toBeInTheDocument();
  });

  it('does not display the literal "你" label', () => {
    render(<UserBubble text="hello" />);
    // 不应有"你"字单独出现作为标签
    expect(screen.queryByText(/^你$/)).toBeNull();
  });

  it('applies right alignment (justify-content: flex-end)', () => {
    const { container } = render(<UserBubble text="hi" />);
    const row = container.querySelector('.msg-row.user-row') as HTMLElement;
    expect(row).toBeTruthy();
    expect(row.style.justifyContent).toBe('flex-end');
  });

  it('applies gray background and border radius to bubble', () => {
    const { container } = render(<UserBubble text="hi" />);
    const bubble = container.querySelector('.user-bubble') as HTMLElement;
    expect(bubble).toBeTruthy();
    // jsdom 将 #f4f4f5 规范化为 rgb(244, 244, 245)
    expect(bubble.style.background).toBe('rgb(244, 244, 245)');
    expect(bubble.style.borderRadius).toBe('10px');
  });

  it('enforces max-width 60% on bubble', () => {
    const { container } = render(<UserBubble text="hi" />);
    const bubble = container.querySelector('.user-bubble') as HTMLElement;
    expect(bubble.style.maxWidth).toBe('60%');
  });

  it('does not parse markdown (renders raw text)', () => {
    render(<UserBubble text="**bold** _italic_" />);
    // 应作为纯文本显示，不应出现 <strong> 或 <em> 标签
    expect(screen.getByText('**bold** _italic_')).toBeInTheDocument();
    expect(document.querySelector('strong')).toBeNull();
    expect(document.querySelector('em')).toBeNull();
  });

  it('preserves whitespace via white-space: pre-wrap', () => {
    const { container } = render(<UserBubble text={'line1\nline2'} />);
    const bubble = container.querySelector('.user-bubble') as HTMLElement;
    expect(bubble.style.whiteSpace).toBe('pre-wrap');
  });

  it('handles empty string', () => {
    const { container } = render(<UserBubble text="" />);
    const bubble = container.querySelector('.user-bubble') as HTMLElement;
    expect(bubble).toBeTruthy();
    expect(bubble.textContent).toBe('');
  });

  it('renders long text without truncation', () => {
    const long = 'a'.repeat(500);
    render(<UserBubble text={long} />);
    expect(screen.getByText(long)).toBeInTheDocument();
  });
});
