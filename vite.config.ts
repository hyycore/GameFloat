import { resolve } from 'path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  root: resolve(__dirname, 'src/renderer'),
  base: './',
  plugins: [react()],
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyOutDir: true,
    target: 'chrome110',
    rollupOptions: {
      input: {
        overlay: resolve(__dirname, 'src/renderer/overlay/index.html'),
        settings: resolve(__dirname, 'src/renderer/settings/index.html')
      },
      output: {
        // 把体积较大的第三方库拆成独立 chunk，主包更小、缓存更稳。
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined
          // CodeMirror 与其语法解析器放在一起（彼此存在循环引用，拆开会产生
          // 「Circular chunk」告警）；它只在代码编辑器打开时按需加载。
          if (id.includes('@codemirror') || id.includes('@lezer') || id.includes('@uiw')) {
            return 'codemirror'
          }
          if (id.includes('react') || id.includes('scheduler')) return 'react'
          return 'vendor'
        }
      }
    },
    // CodeMirror 的懒加载 chunk 约 500 kB，本身按需加载、不影响首屏，
    // 这里放宽默认的 500 kB 告警线。
    chunkSizeWarningLimit: 600
  },
  server: {
    port: 5173,
    strictPort: true
  }
})
