import { fileURLToPath, URL } from 'node:url'

import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

import packageJson from './package.json' with { type: 'json' }

// The development server forwards API requests to a microscope server, by
// default one running locally:
//   cargo run -p microscope-server -- -c configs/simulation.json
// MICROSCOPE_API_TARGET points it elsewhere, such as http://lab-pc:5000.
const apiTarget = process.env.MICROSCOPE_API_TARGET ?? 'http://127.0.0.1:5000'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version),
  },
  server: {
    proxy: {
      '/api': { target: apiTarget, ws: true },
    },
  },
})
