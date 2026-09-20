import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Dev server proxies /api to the FastAPI backend on :8000.
// `npm run build` emits ./dist, which the FastAPI app serves in single-server mode.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: { '/api': 'http://localhost:8000' },
  },
  build: { outDir: 'dist' },
})
