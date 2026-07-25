// Lean4 安装状态 hook：启动时调 /api/lean/check-install 检测，未安装时弹引导 Modal。
//
// 设计：与 useSettings 类似的接口形态，但只读（不支持运行时安装）。
// 用户需自行在终端安装 elan，重启应用后再次检测。
import { useCallback, useEffect, useState } from 'react';

export interface InstallStep {
  platform: string;
  description: string;
  command?: string | null;
  link?: string | null;
}

export interface LeanInstallStatus {
  installed: boolean;
  version: string | null;
  lean_bin: string;
  install_guide: InstallStep[];
}

interface UseLeanInstallResult extends LeanInstallStatus {
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

const ENDPOINT = '/api/lean/check-install';

export function useLeanInstall(): UseLeanInstallResult {
  const [status, setStatus] = useState<LeanInstallStatus>({
    installed: true, // 默认假设已安装，避免启动时闪一下 Modal
    version: null,
    lean_bin: 'lean',
    install_guide: [],
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const resp = await fetch(ENDPOINT);
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}`);
      }
      const data: LeanInstallStatus = await resp.json();
      setStatus(data);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      // 拉取失败时保守地假设已安装，避免误弹 Modal 阻塞用户
      setStatus((prev) => ({ ...prev, installed: true }));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    ...status,
    loading,
    error,
    refresh,
  };
}
