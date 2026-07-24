import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Tauri 桌面壳下：生产构建需用相对路径（base: './'），以便通过 tauri:// 协议加载
// 开发期：仍用默认绝对路径，由 vite dev server 提供资源
const isTauriBuild = process.env.TAURI_ENV_PLATFORM != null;

export default defineConfig({
  plugins: [react()],
  // Tauri 生产构建用相对路径；Web 形态用默认绝对路径
  base: isTauriBuild ? './' : '/',
  server: {
    port: 5173,
    strictPort: true, // Tauri 期望固定的 dev 端口
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // Tauri 推荐：生产构建用较小 chunk 策略
    target: isTauriBuild ? 'es2021' : 'modules',
  },
});
