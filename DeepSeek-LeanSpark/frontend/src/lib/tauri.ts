// Tauri 环境检测与原生 API 封装。
//
// 在 Tauri 桌面壳中运行时，`window.__TAURI_INTERNALS__` 会被注入。
// 我们通过此特征判断是否处于 Tauri 环境，并按需动态导入 @tauri-apps/plugin-dialog。
//
// 这样前端代码在 Web 形态（npm run dev）与 Tauri 形态（cargo tauri dev）下都能工作，
// 不需要分两套构建配置。

/** 是否运行在 Tauri 桌面壳中 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * 打开原生文件夹选择对话框。
 * - Tauri 环境：调用 @tauri-apps/plugin-dialog 的 open({ directory: true })
 * - Web 环境：降级为 window.prompt 输入路径
 *
 * 返回选中的文件夹绝对路径，或 null 表示用户取消。
 */
export async function pickDirectory(initialPath?: string): Promise<string | null> {
  if (isTauri()) {
    try {
      const mod = await import('@tauri-apps/plugin-dialog');
      const selected = await mod.open({
        directory: true,
        multiple: false,
        defaultPath: initialPath,
        title: '选择工作区文件夹',
      });
      if (typeof selected === 'string' && selected.length > 0) {
        return selected;
      }
      return null;
    } catch (e) {
      console.warn('Tauri 文件对话框调用失败，降级到 prompt：', e);
      // 降级到 prompt
    }
  }
  // Web 形态降级
  const input = window.prompt('请输入要打开的工作区文件夹绝对路径：', initialPath ?? '');
  return input && input.trim().length > 0 ? input.trim() : null;
}
