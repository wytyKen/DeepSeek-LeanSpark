// Lean4 安装引导 Modal：检测到未安装 Lean4 时弹出，提供 elan 安装指引。
//
// 与 SettingsModal 不同：
// - 此 Modal 不可强制阻塞（forceOpen 永远 false），用户可关闭继续使用应用
//   原因：用户可能暂时只想用聊天功能，不强制阻塞
// - 不提供"安装"按钮，因为浏览器/Tauri 沙箱无法直接执行系统安装命令
//   用户需在系统终端手动执行命令
import { useEffect, useState } from 'react';
import type { InstallStep, LeanInstallStatus } from '../../hooks/useLeanInstall';

interface Props {
  open: boolean;
  status: LeanInstallStatus;
  onClose: () => void;
}

export function LeanInstallModal({ open, status, onClose }: Props) {
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setCopiedCmd(null);
    }
  }, [open]);

  if (!open) return null;

  // 按平台分组展示步骤
  const allSteps = status.install_guide.filter((s) => s.platform === 'all');
  const platformSteps: InstallStep[] = (() => {
    const ua = navigator.userAgent;
    let platform: string;
    if (ua.includes('Windows')) {
      platform = 'windows';
    } else if (ua.includes('Mac')) {
      platform = 'macos';
    } else {
      platform = 'linux';
    }
    // macOS 和 Linux 共用 curl 命令，统一归到 macos
    const target = platform === 'windows' ? 'windows' : 'macos';
    return status.install_guide.filter((s) => s.platform === target);
  })();

  const handleCopy = async (cmd: string) => {
    try {
      await navigator.clipboard.writeText(cmd);
      setCopiedCmd(cmd);
      setTimeout(() => setCopiedCmd(null), 1500);
    } catch {
      // 静默失败
    }
  };

  return (
    <div
      className="lean-install-modal-overlay"
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0, 0, 0, 0.4)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 999, // 比 SettingsModal (1000) 低，让用户先配置 API Key
      }}
    >
      <div
        className="lean-install-modal"
        style={{
          background: '#fff',
          borderRadius: 8,
          padding: 24,
          width: 560,
          maxWidth: '90vw',
          maxHeight: '90vh',
          overflow: 'auto',
          boxShadow: '0 8px 24px rgba(0, 0, 0, 0.2)',
        }}
      >
        <h2
          style={{
            margin: '0 0 8px',
            fontSize: 18,
            fontWeight: 600,
            color: '#1f2937',
          }}
        >
          检测到未安装 Lean4
        </h2>
        <p style={{ margin: '0 0 16px', fontSize: 13, color: '#666' }}>
          DeepSeek-LeanSpark 需要 Lean4 验证 AI 生成的证明代码。未安装时 Agent
          仍可生成代码，但无法判断证明是否正确。建议按以下步骤安装。
        </p>

        {status.version && (
          <div
            style={{
              fontSize: 12,
              color: '#15803d',
              background: '#f0fdf4',
              padding: '6px 10px',
              borderRadius: 4,
              marginBottom: 12,
            }}
          >
            当前版本：{status.version}
          </div>
        )}

        <div style={{ marginBottom: 16 }}>
          <h3
            style={{
              fontSize: 14,
              fontWeight: 600,
              margin: '0 0 8px',
              color: '#374151',
            }}
          >
            安装步骤
          </h3>
          <ol
            style={{
              margin: 0,
              paddingLeft: 20,
              fontSize: 13,
              lineHeight: 1.7,
              color: '#374151',
            }}
          >
            {allSteps.map((step, i) => (
              <li key={`all-${i}`} style={{ marginBottom: 8 }}>
                <div>{step.description}</div>
                {step.link && (
                  <a
                    href={step.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ color: '#2563eb', fontSize: 12 }}
                  >
                    {step.link}
                  </a>
                )}
                {step.command && (
                  <div
                    style={{
                      marginTop: 4,
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                    }}
                  >
                    <code
                      style={{
                        flex: 1,
                        padding: '4px 8px',
                        background: '#f5f5f5',
                        border: '1px solid #e5e5e5',
                        borderRadius: 4,
                        fontSize: 12,
                        fontFamily: 'ui-monospace, Consolas, monospace',
                        color: '#c7254e',
                        wordBreak: 'break-all',
                      }}
                    >
                      {step.command}
                    </code>
                    <button
                      type="button"
                      onClick={() => handleCopy(step.command!)}
                      style={{
                        fontSize: 11,
                        padding: '4px 8px',
                        background:
                          copiedCmd === step.command ? '#dcfce7' : '#f5f5f5',
                        color: copiedCmd === step.command ? '#15803d' : '#555',
                        border: '1px solid #e5e5e5',
                        borderRadius: 4,
                        cursor: 'pointer',
                        flexShrink: 0,
                      }}
                    >
                      {copiedCmd === step.command ? '已复制' : '复制'}
                    </button>
                  </div>
                )}
              </li>
            ))}
            {platformSteps.map((step, i) => (
              <li key={`plat-${i}`} style={{ marginBottom: 8 }}>
                <div>{step.description}</div>
                {step.command && (
                  <div
                    style={{
                      marginTop: 4,
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                    }}
                  >
                    <code
                      style={{
                        flex: 1,
                        padding: '4px 8px',
                        background: '#f5f5f5',
                        border: '1px solid #e5e5e5',
                        borderRadius: 4,
                        fontSize: 12,
                        fontFamily: 'ui-monospace, Consolas, monospace',
                        color: '#c7254e',
                        wordBreak: 'break-all',
                      }}
                    >
                      {step.command}
                    </code>
                    <button
                      type="button"
                      onClick={() => handleCopy(step.command!)}
                      style={{
                        fontSize: 11,
                        padding: '4px 8px',
                        background:
                          copiedCmd === step.command ? '#dcfce7' : '#f5f5f5',
                        color: copiedCmd === step.command ? '#15803d' : '#555',
                        border: '1px solid #e5e5e5',
                        borderRadius: 4,
                        cursor: 'pointer',
                        flexShrink: 0,
                      }}
                    >
                      {copiedCmd === step.command ? '已复制' : '复制'}
                    </button>
                  </div>
                )}
              </li>
            ))}
          </ol>
        </div>

        <div
          style={{
            padding: 12,
            background: '#fef3c7',
            border: '1px solid #fde68a',
            borderRadius: 4,
            fontSize: 12,
            color: '#92400e',
            marginBottom: 16,
            lineHeight: 1.5,
          }}
        >
          <strong>提示：</strong>
          安装完成后请重启本应用，让新的 PATH 生效。若重启后仍检测不到，
          可在 .env 中设置 <code>LEAN_BIN_PATH</code> 指向 lean 二进制的绝对路径。
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: '6px 14px',
              fontSize: 13,
              background: '#f5f5f5',
              border: '1px solid #e5e5e5',
              borderRadius: 4,
              cursor: 'pointer',
              color: '#555',
            }}
          >
            稍后再说
          </button>
        </div>
      </div>
    </div>
  );
}
