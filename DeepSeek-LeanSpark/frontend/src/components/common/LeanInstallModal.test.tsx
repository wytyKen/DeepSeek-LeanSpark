// LeanInstallModal 组件测试
// 验证：条件渲染、标题展示、安装步骤渲染、复制命令、关闭按钮、平台过滤。
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { LeanInstallModal } from './LeanInstallModal';
import type { LeanInstallStatus } from '../../hooks/useLeanInstall';

function makeStatus(overrides: Partial<LeanInstallStatus> = {}): LeanInstallStatus {
  return {
    installed: false,
    version: null,
    lean_bin: 'lean',
    install_guide: [
      {
        platform: 'all',
        description: 'Lean4 通过 elan 安装。',
        command: null,
        link: 'https://github.com/leanprover/elan',
      },
      {
        platform: 'windows',
        description: 'Windows PowerShell 命令。',
        command: 'Invoke-WebRequest ... ; ./elan-init.ps1',
        link: null,
      },
      {
        platform: 'macos',
        description: 'macOS/Linux curl 命令。',
        command: 'curl ... | sh',
        link: null,
      },
      {
        platform: 'all',
        description: '重启应用让 PATH 生效。',
        command: null,
        link: null,
      },
      {
        platform: 'all',
        description: '验证：运行 lean --version。',
        command: 'lean --version',
        link: null,
      },
    ],
    ...overrides,
  };
}

describe('LeanInstallModal', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.mocked(navigator.clipboard.writeText).mockClear();
    vi.mocked(navigator.clipboard.writeText).mockResolvedValue(undefined);
  });

  it('returns null when open=false', () => {
    const { container } = render(
      <LeanInstallModal open={false} status={makeStatus()} onClose={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders title when open=true', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(screen.getByText('检测到未安装 Lean4')).toBeInTheDocument();
  });

  it('renders description text', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(
      screen.getByText(/需要 Lean4 验证 AI 生成的证明代码/),
    ).toBeInTheDocument();
  });

  it('renders all-platform steps', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(screen.getByText('Lean4 通过 elan 安装。')).toBeInTheDocument();
    expect(screen.getByText('重启应用让 PATH 生效。')).toBeInTheDocument();
    expect(screen.getByText(/验证：运行 lean --version/)).toBeInTheDocument();
  });

  it('renders elan link', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    const link = screen.getByRole('link', { name: 'https://github.com/leanprover/elan' });
    expect(link).toHaveAttribute('href', 'https://github.com/leanprover/elan');
    expect(link).toHaveAttribute('target', '_blank');
  });

  it('renders platform-specific steps based on userAgent (Windows)', () => {
    // 模拟 Windows userAgent
    const originalUA = navigator.userAgent;
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
      configurable: true,
    });
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(screen.getByText('Windows PowerShell 命令。')).toBeInTheDocument();
    expect(screen.queryByText('macOS/Linux curl 命令。')).toBeNull();
    Object.defineProperty(navigator, 'userAgent', {
      value: originalUA,
      configurable: true,
    });
  });

  it('renders platform-specific steps based on userAgent (macOS)', () => {
    const originalUA = navigator.userAgent;
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
      configurable: true,
    });
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(screen.getByText('macOS/Linux curl 命令。')).toBeInTheDocument();
    expect(screen.queryByText('Windows PowerShell 命令。')).toBeNull();
    Object.defineProperty(navigator, 'userAgent', {
      value: originalUA,
      configurable: true,
    });
  });

  it('renders command code blocks for steps with commands', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    // 至少应有两个 command code block（lean --version 和 PowerShell/curl）
    const codeBlocks = document.querySelectorAll('code');
    const commands = Array.from(codeBlocks).map((el) => el.textContent);
    expect(commands).toContain('lean --version');
  });

  it('calls onClose when close button clicked', () => {
    const onClose = vi.fn();
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={onClose} />);
    fireEvent.click(screen.getByRole('button', { name: '稍后再说' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls navigator.clipboard.writeText when copy button clicked', async () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    // 找到 lean --version 命令的复制按钮（在该 code block 旁边）
    const copyButtons = screen.getAllByRole('button', { name: '复制' });
    expect(copyButtons.length).toBeGreaterThan(0);
    fireEvent.click(copyButtons[0]);
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalled();
    });
  });

  it('shows "已复制" after successful copy', async () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    const copyButtons = screen.getAllByRole('button', { name: '复制' });
    fireEvent.click(copyButtons[0]);
    await waitFor(() => {
      expect(screen.getAllByRole('button', { name: '已复制' }).length).toBeGreaterThan(0);
    });
  });

  it('renders LEAN_BIN_PATH hint in tip box', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(screen.getByText(/LEAN_BIN_PATH/)).toBeInTheDocument();
  });

  it('renders version banner when status.version is set', () => {
    const status = makeStatus({ version: 'Lean version 4.0.0' });
    render(<LeanInstallModal open={true} status={status} onClose={vi.fn()} />);
    expect(screen.getByText(/Lean version 4.0.0/)).toBeInTheDocument();
  });

  it('does not render version banner when version is null', () => {
    const status = makeStatus({ version: null });
    render(<LeanInstallModal open={true} status={status} onClose={vi.fn()} />);
    expect(screen.queryByText(/Lean version/)).toBeNull();
  });

  it('renders "稍后再说" close button', () => {
    render(<LeanInstallModal open={true} status={makeStatus()} onClose={vi.fn()} />);
    expect(screen.getByRole('button', { name: '稍后再说' })).toBeInTheDocument();
  });
});
