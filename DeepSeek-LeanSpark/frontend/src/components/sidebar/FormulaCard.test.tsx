// FormulaCard 组件测试
// 验证：KaTeX 异步渲染、点击回调、复制按钮、source 显示。
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { FormulaCard } from './FormulaCard';

describe('FormulaCard', () => {
  it('renders loading state then KaTeX HTML', async () => {
    render(<FormulaCard latex="a + b" />);
    await waitFor(() => {
      const rendered = document.querySelector('.formula-rendered');
      expect(rendered?.innerHTML).not.toBe('');
    });
  });

  it('renders KaTeX with span.katex class', async () => {
    render(<FormulaCard latex="x^2" />);
    await waitFor(() => {
      expect(document.querySelector('.katex')).toBeInTheDocument();
    });
  });

  it('calls onClick with latex when card clicked', async () => {
    const onClick = vi.fn();
    render(<FormulaCard latex="E=mc^2" onClick={onClick} />);
    await waitFor(() => expect(document.querySelector('.katex')).toBeInTheDocument());
    fireEvent.click(screen.getByText('复制 LaTeX').parentElement?.parentElement as HTMLElement);
    // 点卡片本身（非复制按钮）
    const card = document.querySelector('.formula-card') as HTMLElement;
    fireEvent.click(card);
    expect(onClick).toHaveBeenCalledWith('E=mc^2');
  });

  it('shows source label when provided', async () => {
    render(<FormulaCard latex="x" source="来自第 1 轮对话" />);
    await waitFor(() => expect(document.querySelector('.katex')).toBeInTheDocument());
    expect(screen.getByText('来自第 1 轮对话')).toBeInTheDocument();
  });

  it('shows copy button always', () => {
    render(<FormulaCard latex="x" />);
    expect(screen.getByRole('button', { name: '复制 LaTeX' })).toBeInTheDocument();
  });

  it('copy button stopPropagation prevents card click', async () => {
    const onClick = vi.fn();
    render(<FormulaCard latex="x" onClick={onClick} />);
    // 用 act 包裹点击，确保 CopyButton 内部的 setCopied 异步状态更新被捕获
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: '复制 LaTeX' }));
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(onClick).not.toHaveBeenCalled();
  });

  it('handles invalid latex gracefully (no crash)', async () => {
    render(<FormulaCard latex="\\invalidCmd{bad" />);
    await waitFor(() => {
      const rendered = document.querySelector('.formula-rendered');
      // KaTeX throwOnError:false 会渲染错误颜色，但不会崩溃
      expect(rendered).toBeTruthy();
    });
  });

  it('re-renders when latex prop changes', async () => {
    const { rerender } = render(<FormulaCard latex="a" />);
    await waitFor(() => expect(document.querySelector('.katex')).toBeInTheDocument());
    rerender(<FormulaCard latex="b" />);
    await waitFor(() => {
      expect(document.querySelector('.katex')).toBeInTheDocument();
    });
  });

  it('does not crash if katex module fails to load', async () => {
    // 模拟 import 失败：暂时覆盖 dynamic import
    const original = await import('katex');
    vi.doMock('katex', () => { throw new Error('module not found'); });
    render(<FormulaCard latex="x" />);
    await waitFor(() => {
      const rendered = document.querySelector('.formula-rendered');
      expect(rendered).toBeTruthy();
    });
    vi.doUnmock('katex');
    // 确保不污染后续测试
    await original.renderToString('x', { throwOnError: false });
  });
});
