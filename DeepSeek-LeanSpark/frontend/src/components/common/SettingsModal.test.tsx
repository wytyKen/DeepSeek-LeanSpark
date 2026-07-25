// SettingsModal 组件测试
// 验证：条件渲染、空 key 阻止提交、提交回调、成功反馈、forceOpen 隐藏取消按钮、显示/隐藏 key。
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { SettingsModal } from './SettingsModal';

describe('SettingsModal', () => {
  it('returns null when open=false', () => {
    const { container } = render(
      <SettingsModal
        open={false}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders title and inputs when open=true', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByText('DeepSeek API Key 设置')).toBeInTheDocument();
    expect(screen.getByLabelText('API Key')).toBeInTheDocument();
    expect(screen.getByLabelText('模型名')).toBeInTheDocument();
  });

  it('shows placeholder for current model', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-chat"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    const modelInput = screen.getByLabelText('模型名') as HTMLInputElement;
    expect(modelInput.placeholder).toBe('deepseek-chat');
  });

  it('disables save button when api key is empty', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    const saveBtn = screen.getByRole('button', { name: '保存' });
    expect(saveBtn).toBeDisabled();
  });

  it('shows error when submitting empty key (defensive)', async () => {
    // 即使按钮被禁用， handleSubmit 内部也会防御性检查
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    // 直接调 form submit（绕过 disabled button）
    const form = screen.getByText('API Key').closest('form')!;
    fireEvent.submit(form);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('API Key 不能为空');
    });
  });

  it('calls onSubmit with trimmed api key and model', async () => {
    const onSubmit = vi.fn().mockResolvedValue({
      success: true,
      model: 'deepseek-reasoner',
    });
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={onSubmit}
        onClose={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText('API Key'), {
      target: { value: 'sk-test-key' },
    });
    fireEvent.change(screen.getByLabelText('模型名'), {
      target: { value: 'deepseek-reasoner' },
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));
    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith('sk-test-key', 'deepseek-reasoner');
    });
  });

  it('calls onSubmit without model when model field is empty', async () => {
    const onSubmit = vi.fn().mockResolvedValue({
      success: true,
      model: 'deepseek-v4-flash',
    });
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={onSubmit}
        onClose={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText('API Key'), {
      target: { value: 'sk-test' },
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));
    await waitFor(() => {
      // model 字段为空时传 undefined，让后端沿用当前模型
      expect(onSubmit).toHaveBeenCalledWith('sk-test', undefined);
    });
  });

  it('shows success message after successful submit', async () => {
    const onSubmit = vi.fn().mockResolvedValue({
      success: true,
      model: 'deepseek-v4-flash',
    });
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={onSubmit}
        onClose={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText('API Key'), {
      target: { value: 'sk-test' },
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('已配置，当前模型：deepseek-v4-flash');
    });
  });

  it('shows error message when submit fails', async () => {
    const onSubmit = vi.fn().mockResolvedValue({
      success: false,
      model: 'deepseek-v4-flash',
      error: '后端校验失败',
    });
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={onSubmit}
        onClose={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText('API Key'), {
      target: { value: 'sk-test' },
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('后端校验失败');
    });
  });

  it('hides cancel button when forceOpen=true', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={true}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.queryByRole('button', { name: '取消' })).toBeNull();
  });

  it('shows cancel button when forceOpen=false', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: '取消' })).toBeInTheDocument();
  });

  it('calls onClose when cancel button clicked', () => {
    const onClose = vi.fn();
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={onClose}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '取消' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('toggles api key visibility when show/hide button clicked', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    const apiKeyInput = screen.getByLabelText('API Key') as HTMLInputElement;
    expect(apiKeyInput.type).toBe('password');
    // 初始按钮 aria-label 为"显示 API Key"
    const showBtn = screen.getByLabelText('显示 API Key');
    fireEvent.click(showBtn);
    expect(apiKeyInput.type).toBe('text');
    // 切换后按钮 aria-label 变为"隐藏 API Key"
    expect(screen.getByLabelText('隐藏 API Key')).toBeInTheDocument();
  });

  it('disables save button while submitting', async () => {
    let resolveSubmit: (v: { success: boolean; model: string }) => void = () => {};
    const onSubmit = vi.fn().mockReturnValue(
      new Promise<{ success: boolean; model: string }>((resolve) => {
        resolveSubmit = resolve;
      }),
    );
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={onSubmit}
        onClose={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText('API Key'), {
      target: { value: 'sk-test' },
    });
    const saveBtn = screen.getByRole('button', { name: '保存' });
    fireEvent.click(saveBtn);
    await waitFor(() => {
      // 提交中按钮文案变为"提交中..."
      expect(screen.getByRole('button', { name: '提交中...' })).toBeDisabled();
    });
    resolveSubmit({ success: true, model: 'deepseek-v4-flash' });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: '保存' })).toBeInTheDocument();
    });
  });

  it('renders deepseek platform link', () => {
    render(
      <SettingsModal
        open={true}
        currentModel="deepseek-v4-flash"
        forceOpen={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    const link = screen.getByRole('link', { name: 'platform.deepseek.com' });
    expect(link).toHaveAttribute('href', 'https://platform.deepseek.com');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });
});
