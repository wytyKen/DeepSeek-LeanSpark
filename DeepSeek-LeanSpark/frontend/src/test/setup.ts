// Vitest 全局 setup：注册 @testing-library/jest-dom 匹配器 + 模拟剪贴板 API。
//
// jsdom 不实现 navigator.clipboard，CopyButton 等组件依赖它，需在测试前补桩。
import '@testing-library/jest-dom';
import { vi } from 'vitest';

// 模拟 navigator.clipboard.writeText（CopyButton / FormulaCard / LatexModal 依赖）
// jsdom 29+ 可能存在部分 clipboard 实现（writeText 抛 NotAllowedError），统一覆盖为可控 mock。
Object.defineProperty(navigator, 'clipboard', {
  value: {
    writeText: vi.fn().mockResolvedValue(undefined),
    readText: vi.fn().mockResolvedValue(''),
  },
  writable: true,
  configurable: true,
});

// jsdom 不实现 window.matchMedia（部分组件库依赖，预防性补桩）
if (!window.matchMedia) {
  Object.defineProperty(window, 'matchMedia', {
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }),
    writable: true,
  });
}
