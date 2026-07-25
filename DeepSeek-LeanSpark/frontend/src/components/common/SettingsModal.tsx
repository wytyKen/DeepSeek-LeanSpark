// 设置 Modal：让用户在应用内配置 DeepSeek API Key。
//
// 触发场景：
// 1. 应用启动时调 GET /api/settings/api-key，若 configured=false，App.tsx 强制弹此 Modal
// 2. 用户点击 header 的"设置"按钮，主动打开此 Modal 修改 key 或模型
//
// 安全设计：
// - API Key 用 <input type="password">，避免肩窥
// - 不将 key 写入 localStorage（避免明文泄漏），每次重启需重新输入
// - 提交后调 POST /api/settings/api-key，后端 replace_client 注入运行时客户端
//
// 持久化策略：
// - 开发者：通过 .env 配置（DEEPSEEK_API_KEY），后端启动时自动加载
// - 终端用户：每次启动应用时通过此 Modal 输入（不持久化，重启后需重新配置）
//   这是安全与便利的折中：避免明文存储 key
import { useEffect, useState } from 'react';
import type { SetKeyResult } from '../../hooks/useSettings';

interface Props {
  /** 是否打开 */
  open: boolean;
  /** 当前生效模型（用于占位提示） */
  currentModel: string;
  /** 是否强制模式（未配置时不可关闭） */
  forceOpen: boolean;
  /** 提交回调，返回后端响应 */
  onSubmit: (apiKey: string, model?: string) => Promise<SetKeyResult>;
  /** 关闭回调（forceOpen=true 时关闭按钮不显示） */
  onClose: () => void;
}

export function SettingsModal({
  open,
  currentModel,
  forceOpen,
  onSubmit,
  onClose,
}: Props) {
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [resultMsg, setResultMsg] = useState<string | null>(null);
  const [isError, setIsError] = useState(false);

  // Modal 重新打开时重置临时状态
  useEffect(() => {
    if (open) {
      setApiKey('');
      setModel('');
      setShowKey(false);
      setSubmitting(false);
      setResultMsg(null);
      setIsError(false);
    }
  }, [open]);

  if (!open) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (submitting) return;
    if (!apiKey.trim()) {
      setResultMsg('API Key 不能为空');
      setIsError(true);
      return;
    }
    setSubmitting(true);
    setResultMsg(null);
    setIsError(false);
    try {
      const result = await onSubmit(apiKey, model || undefined);
      if (result.success) {
        setResultMsg(`已配置，当前模型：${result.model}`);
        setIsError(false);
        // 成功后短暂延迟关闭，让用户看到反馈
        setTimeout(() => {
          if (!forceOpen) onClose();
        }, 600);
      } else {
        setResultMsg(result.error ?? '设置失败');
        setIsError(true);
      }
    } catch (e) {
      setResultMsg(e instanceof Error ? e.message : String(e));
      setIsError(true);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="settings-modal-overlay"
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
        zIndex: 1000,
      }}
    >
      <div
        className="settings-modal"
        style={{
          background: '#fff',
          borderRadius: 8,
          padding: 24,
          width: 480,
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
          DeepSeek API Key 设置
        </h2>
        <p style={{ margin: '0 0 16px', fontSize: 13, color: '#666' }}>
          {forceOpen
            ? '检测到尚未配置 API Key，请在此填入后开始使用。Key 仅保存在内存中，重启后需重新输入。'
            : '修改 API Key 或模型。提交后立即生效，无需重启应用。'}
        </p>

        <form onSubmit={handleSubmit}>
          <div style={{ marginBottom: 12 }}>
            <label
              htmlFor="settings-api-key"
              style={{
                display: 'block',
                fontSize: 13,
                color: '#374151',
                marginBottom: 4,
              }}
            >
              API Key
            </label>
            <div style={{ display: 'flex', gap: 8 }}>
              <input
                id="settings-api-key"
                type={showKey ? 'text' : 'password'}
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-xxxxxxxx"
                autoComplete="off"
                spellCheck={false}
                style={{
                  flex: 1,
                  padding: '6px 10px',
                  fontSize: 13,
                  border: '1px solid #d1d5db',
                  borderRadius: 4,
                  fontFamily: 'ui-monospace, Consolas, monospace',
                }}
                aria-label="API Key"
              />
              <button
                type="button"
                onClick={() => setShowKey((v) => !v)}
                style={{
                  padding: '6px 10px',
                  fontSize: 12,
                  background: '#f5f5f5',
                  border: '1px solid #e5e5e5',
                  borderRadius: 4,
                  cursor: 'pointer',
                  color: '#555',
                }}
                aria-label={showKey ? '隐藏 API Key' : '显示 API Key'}
              >
                {showKey ? '隐藏' : '显示'}
              </button>
            </div>
          </div>

          <div style={{ marginBottom: 16 }}>
            <label
              htmlFor="settings-model"
              style={{
                display: 'block',
                fontSize: 13,
                color: '#374151',
                marginBottom: 4,
              }}
            >
              模型（可选）
            </label>
            <input
              id="settings-model"
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder={currentModel || 'deepseek-v4-flash'}
              spellCheck={false}
              style={{
                width: '100%',
                padding: '6px 10px',
                fontSize: 13,
                border: '1px solid #d1d5db',
                borderRadius: 4,
              }}
              aria-label="模型名"
            />
            <div style={{ fontSize: 11, color: '#999', marginTop: 4 }}>
              留空则沿用当前模型（{currentModel}）。候选：deepseek-v4-flash / deepseek-v4-pro
            </div>
          </div>

          {resultMsg && (
            <div
              role="alert"
              style={{
                fontSize: 13,
                padding: '6px 10px',
                marginBottom: 12,
                borderRadius: 4,
                background: isError ? '#fef2f2' : '#f0fdf4',
                color: isError ? '#b91c1c' : '#15803d',
                border: `1px solid ${isError ? '#fecaca' : '#bbf7d0'}`,
              }}
            >
              {resultMsg}
            </div>
          )}

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
            {!forceOpen && (
              <button
                type="button"
                onClick={onClose}
                disabled={submitting}
                style={{
                  padding: '6px 14px',
                  fontSize: 13,
                  background: '#f5f5f5',
                  border: '1px solid #e5e5e5',
                  borderRadius: 4,
                  cursor: submitting ? 'not-allowed' : 'pointer',
                  color: '#555',
                }}
              >
                取消
              </button>
            )}
            <button
              type="submit"
              disabled={submitting || !apiKey.trim()}
              style={{
                padding: '6px 14px',
                fontSize: 13,
                background:
                  submitting || !apiKey.trim() ? '#9ca3af' : '#2563eb',
                color: '#fff',
                border: 'none',
                borderRadius: 4,
                cursor:
                  submitting || !apiKey.trim() ? 'not-allowed' : 'pointer',
                fontWeight: 500,
              }}
            >
              {submitting ? '提交中...' : '保存'}
            </button>
          </div>
        </form>

        <div
          style={{
            marginTop: 16,
            paddingTop: 12,
            borderTop: '1px solid #f0f0f0',
            fontSize: 11,
            color: '#999',
            lineHeight: 1.5,
          }}
        >
          <strong>获取 API Key：</strong>
          <a
            href="https://platform.deepseek.com"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: '#2563eb', marginLeft: 4 }}
          >
            platform.deepseek.com
          </a>
          <br />
          <strong>安全说明：</strong>Key 仅保存在内存中，应用关闭后不会被持久化。
        </div>
      </div>
    </div>
  );
}
