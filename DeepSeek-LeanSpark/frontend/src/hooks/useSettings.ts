// 设置 hook：管理 API Key 配置状态，封装 /api/settings/api-key 接口调用。
//
// 设计要点：
// - 后端是 API Key 的唯一权威源：前端启动时调 GET /api/settings/api-key 查 configured 字段
// - API Key 本身不存 localStorage（避免明文泄漏）；只存"用户上次是否选择记住"的偏好
// - 用户在 Modal 输入 key 后，调 POST /api/settings/api-key，后端 replace_client 注入
// - 重启后状态丢失（除非 .env 已配置 key，此时后端会自动加载）
import { useCallback, useEffect, useState } from 'react';

export interface ApiKeyStatus {
  configured: boolean;
  model: string;
  /** 是否正在加载状态（首次 GET 请求中） */
  loading: boolean;
  /** 上次操作错误信息（仅供 UI 显示，不区分网络错误与后端错误） */
  error: string | null;
}

export interface SetKeyResult {
  success: boolean;
  model: string;
  error?: string;
}

const STATUS_ENDPOINT = '/api/settings/api-key';

/**
 * 管理 API Key 配置状态。
 *
 * 启动时自动调 GET /api/settings/api-key 查 configured。
 * - configured=false 时前端应弹 Modal 强制配置
 * - 用户输入 key 后调 setApiKey，成功后 configured 变为 true
 */
export function useSettings(): ApiKeyStatus & {
  /** 重新拉取配置状态（如 Modal 关闭后想刷新） */
  refresh: () => Promise<void>;
  /** 提交 API Key 到后端。返回是否成功。model 可选，不传则沿用后端当前模型。 */
  setApiKey: (apiKey: string, model?: string) => Promise<SetKeyResult>;
} {
  const [configured, setConfigured] = useState(false);
  const [model, setModel] = useState('deepseek-v4-flash');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const resp = await fetch(STATUS_ENDPOINT);
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}`);
      }
      const data: { configured: boolean; model: string } = await resp.json();
      setConfigured(data.configured);
      setModel(data.model);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      // 拉取失败时保守地认为未配置，强制弹 Modal
      setConfigured(false);
    } finally {
      setLoading(false);
    }
  }, []);

  const setApiKey = useCallback(
    async (apiKey: string, modelOverride?: string): Promise<SetKeyResult> => {
      setError(null);
      const trimmed = apiKey.trim();
      if (!trimmed) {
        const msg = 'API Key 不能为空';
        setError(msg);
        return { success: false, model, error: msg };
      }
      try {
        const body: { api_key: string; model?: string } = { api_key: trimmed };
        if (modelOverride && modelOverride.trim()) {
          body.model = modelOverride.trim();
        }
        const resp = await fetch(STATUS_ENDPOINT, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        if (!resp.ok) {
          throw new Error(`HTTP ${resp.status}`);
        }
        const data: { success: boolean; model: string; error?: string } =
          await resp.json();
        if (data.success) {
          setConfigured(true);
          setModel(data.model);
          return { success: true, model: data.model };
        }
        setError(data.error ?? '设置失败');
        return {
          success: false,
          model: data.model,
          error: data.error,
        };
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        return { success: false, model, error: msg };
      }
    },
    [model],
  );

  // 启动时拉取一次状态
  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    configured,
    model,
    loading,
    error,
    refresh,
    setApiKey,
  };
}
