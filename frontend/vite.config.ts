import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  base: '/chat/',
  plugins: [vue()],
  server: {
    proxy: {
      '/auth': {
        target: 'http://127.0.0.1:4008',
        changeOrigin: true,
      },
    },
  },
})
