// useLeanInstall hook 测试
// 验证：启动时拉取状态、installed=true/false 路径、loading 状态、fetch 失败回退。
import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useLeanInstall } from './useLeanInstall';

const SAMPLE_STATUS = {
  installed: false,
  version: null,
  lean_bin: 'lean',
  install_guide: [
    {
      platform: 'all',
      description: 'step1',
      command: null,
      link: 'https://example.com',
    },
  ],
};

describe('useLeanInstall', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('has loading=true initially', () => {
    vi.spyOn(globalThis, 'fetch').mockReturnValue(
      new Promise<Response>(() => {
        // 永不 resolve
      }),
    );
    const { result } = renderHook(() => useLeanInstall());
    expect(result.current.loading).toBe(true);
  });

  it('defaults to installed=true before fetch resolves (avoid flash)', () => {
    vi.spyOn(globalThis, 'fetch').mockReturnValue(
      new Promise<Response>(() => {
        // 永不 resolve
      }),
    );
    const { result } = renderHook(() => useLeanInstall());
    expect(result.current.installed).toBe(true);
  });

  it('fetches status on mount and sets installed=false when backend returns false', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(JSON.stringify(SAMPLE_STATUS), { status: 200 }),
    );

    const { result } = renderHook(() => useLeanInstall());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.installed).toBe(false);
    expect(result.current.install_guide).toHaveLength(1);
    expect(result.current.error).toBeNull();
  });

  it('sets installed=true when backend returns installed=true', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          installed: true,
          version: 'Lean version 4.0.0',
          lean_bin: '/usr/local/bin/lean',
          install_guide: [],
        }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useLeanInstall());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.installed).toBe(true);
    expect(result.current.version).toBe('Lean version 4.0.0');
    expect(result.current.lean_bin).toBe('/usr/local/bin/lean');
  });

  it('sets error and falls back to installed=true when fetch throws', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('network down'));

    const { result } = renderHook(() => useLeanInstall());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toBe('network down');
    // 拉取失败时保守地假设已安装，避免误弹 Modal 阻塞用户
    expect(result.current.installed).toBe(true);
  });

  it('sets error when fetch returns non-200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response('err', { status: 500 }),
    );

    const { result } = renderHook(() => useLeanInstall());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toBe('HTTP 500');
    expect(result.current.installed).toBe(true);
  });

  it('refresh re-fetches status from backend', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ installed: false, version: null, lean_bin: 'lean', install_guide: [] }),
        { status: 200 },
      ),
    );
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          installed: true,
          version: 'Lean version 4.1.0',
          lean_bin: 'lean',
          install_guide: [],
        }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useLeanInstall());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.installed).toBe(false);

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.installed).toBe(true);
    expect(result.current.version).toBe('Lean version 4.1.0');
  });
});
