import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: {
    host: '0.0.0.0',
    port: 43691,
    allowedHosts: ['amstock.amsors.top','amstock-dev.amsors.top'],
    proxy: {
      '/api': 'http://127.0.0.1:3000',
      '/images': 'http://127.0.0.1:3000',
    },
  },
})
