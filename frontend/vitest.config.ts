import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

/// 单测配置（与 vite.config.ts 分开：测试不需要 Tailwind 插件，jsdom 环境即可）
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
