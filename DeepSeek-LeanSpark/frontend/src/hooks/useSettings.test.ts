// useSettings hook 测试
// 验证：启动时拉取状态、setApiKey 成功/失败路径、loading 状态切换、空 key 防御。
import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useSettings, type SetKeyResult } from './useSettings';

describe('useSettings', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetches status on mount and sets configured=true when backend returns configured', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(JSON.stringify({ configured: true, model: 'deepseek-chat' }), {
        status: 200,
      }),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.configured).toBe(true);
    expect(result.current.model).toBe('deepseek-chat');
    expect(result.current.error).toBeNull();
  });

  it('sets configured=false when backend returns configured=false', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.configured).toBe(false);
    expect(result.current.model).toBe('deepseek-v4-flash');
  });

  it('sets error and configured=false when fetch fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('network down'));

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.configured).toBe(false);
    expect(result.current.error).toBe('network down');
  });

  it('sets error when fetch returns non-200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response('internal error', { status: 500 }),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.configured).toBe(false);
    expect(result.current.error).toBe('HTTP 500');
  });

  it('setApiKey posts to endpoint and updates configured=true on success', async () => {
    // GET 启动请求
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );
    // POST 设置请求
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ success: true, model: 'deepseek-reasoner' }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    let setResult: SetKeyResult | undefined;
    await act(async () => {
      setResult = await result.current.setApiKey('sk-new', 'deepseek-reasoner');
    });

    expect(setResult!.success).toBe(true);
    expect(setResult!.model).toBe('deepseek-reasoner');
    expect(result.current.configured).toBe(true);
    expect(result.current.model).toBe('deepseek-reasoner');

    // 验证 POST 请求体
    const postCall = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls.find(
      (call) => (call[1] as RequestInit)?.method === 'POST',
    );
    expect(postCall).toBeDefined();
    expect((postCall![1] as RequestInit).body).toBe(
      JSON.stringify({ api_key: 'sk-new', model: 'deepseek-reasoner' }),
    );
  });

  it('setApiKey does not send model field when modelOverride is empty', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ success: true, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.setApiKey('sk-new');
    });

    const postCall = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls.find(
      (call) => (call[1] as RequestInit)?.method === 'POST',
    );
    expect(postCall).toBeDefined();
    expect((postCall![1] as RequestInit).body).toBe(
      JSON.stringify({ api_key: 'sk-new' }),
    );
  });

  it('setApiKey returns failure when backend returns success=false', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          success: false,
          model: 'deepseek-v4-flash',
          error: '后端校验失败',
        }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    let setResult: SetKeyResult | undefined;
    await act(async () => {
      setResult = await result.current.setApiKey('sk-bad');
    });

    expect(setResult!.success).toBe(false);
    expect(setResult!.error).toBe('后端校验失败');
    expect(result.current.configured).toBe(false);
    expect(result.current.error).toBe('后端校验失败');
  });

  it('setApiKey returns failure when POST throws', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );
    vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('post failed'));

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    let setResult: SetKeyResult | undefined;
    await act(async () => {
      setResult = await result.current.setApiKey('sk-key');
    });

    expect(setResult!.success).toBe(false);
    expect(setResult!.error).toBe('post failed');
    expect(result.current.error).toBe('post failed');
  });

  it('setApiKey returns failure for empty api key without calling fetch POST', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    let setResult: SetKeyResult | undefined;
    await act(async () => {
      setResult = await result.current.setApiKey('   ');
    });

    expect(setResult!.success).toBe(false);
    expect(setResult!.error).toBe('API Key 不能为空');
    // 不应有 POST 调用
    const postCalls = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls.filter(
      (call) => (call[1] as RequestInit)?.method === 'POST',
    );
    expect(postCalls).toHaveLength(0);
  });

  it('refresh re-fetches status from backend', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: false, model: 'deepseek-v4-flash' }),
        { status: 200 },
      ),
    );
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({ configured: true, model: 'deepseek-chat' }),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.configured).toBe(false);

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.configured).toBe(true);
    expect(result.current.model).toBe('deepseek-chat');
  });

  it('has loading=true initially', () => {
    vi.spyOn(globalThis, 'fetch').mockReturnValue(
      new Promise<Response>(() => {
        // 永不 resolve，测试初始状态
      }),
    );

    const { result } = renderHook(() => useSettings());
    expect(result.current.loading).toBe(true);
  });
});
