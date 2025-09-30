import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import { resolve } from 'path';

export default defineConfig({
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
  ],
  server: {
    port: 3000,
    open: true,
    fs: {
      // Allow serving files from the parent directory (pkg folder)
      allow: ['..'],
    },
  },
  resolve: {
    alias: {
      'finstack-wasm': resolve(__dirname, '../pkg'),
    },
  },
  optimizeDeps: {
    exclude: ['finstack-wasm'],
  },
  build: {
    target: 'esnext',
  },
  worker: {
    format: 'es',
  },
});
