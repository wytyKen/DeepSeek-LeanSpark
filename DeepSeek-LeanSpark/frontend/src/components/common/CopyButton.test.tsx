// CopyButton 组件测试
// 验证：默认标签、自定义标签、点击复制、已复制反馈、大小样式。
import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { CopyButton } from './CopyButton';

// 点击并在同一 act 内 flush 微任务，确保 setCopied 状态更新被 act 捕获
async function clickAndFlush(button: HTMLElement) {
  await act(async () => {
    fireEvent.click(button);
    // 多次 flush 处理 await writeText 解析 + handleCopy 续行的微任务链
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe('CopyButton', () => {
  beforeEach(() => {
    vi.mocked(navigator.clipboard.writeText).mockClear();
    vi.mocked(navigator.clipboard.writeText).mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders default label "复制"', () => {
    render(<CopyButton text="hello" />);
    expect(screen.getByRole('button', { name: '复制' })).toBeInTheDocument();
  });

  it('renders custom label', () => {
    render(<CopyButton text="hello" label="复制 LaTeX" />);
    expect(screen.getByRole('button', { name: '复制 LaTeX' })).toBeInTheDocument();
  });

  it('calls navigator.clipboard.writeText with the provided text on click', async () => {
    render(<CopyButton text="x^2 + y^2" />);
    await clickAndFlush(screen.getByRole('button'));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('x^2 + y^2');
  });

  it('shows "已复制" after successful copy', async () => {
    render(<CopyButton text="hi" />);
    await clickAndFlush(screen.getByRole('button'));
    expect(screen.getByRole('button', { name: '已复制' })).toBeInTheDocument();
  });

  it('reverts to original label after 1500ms', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(<CopyButton text="hi" label="复制" />);
    await clickAndFlush(screen.getByRole('button'));
    expect(screen.getByRole('button', { name: '已复制' })).toBeInTheDocument();
    act(() => { vi.advanceTimersByTime(1500); });
    expect(screen.getByRole('button', { name: '复制' })).toBeInTheDocument();
  });

  it('applies small size styles by default', () => {
    render(<CopyButton text="hi" />);
    const btn = screen.getByRole('button');
    expect(btn.style.fontSize).toBe('11px');
    expect(btn.style.padding).toBe('2px 6px');
  });

  it('applies medium size styles when size="md"', () => {
    render(<CopyButton text="hi" size="md" />);
    const btn = screen.getByRole('button');
    expect(btn.style.fontSize).toBe('13px');
    expect(btn.style.padding).toBe('4px 10px');
  });

  it('applies green background after copy', async () => {
    render(<CopyButton text="hi" />);
    const btn = screen.getByRole('button');
    await clickAndFlush(btn);
    expect(screen.getByRole('button', { name: '已复制' })).toBeInTheDocument();
    // jsdom 将 #dcfce7 规范化为 rgb 形式
    expect(btn.style.background).toBe('rgb(220, 252, 231)');
    expect(btn.style.color).toBe('rgb(21, 128, 61)');
  });
});
