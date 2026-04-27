import { defineConfig } from 'vite';

export default defineConfig({
  base: './',
  server: {
    fs: {
      // Allow Vite to serve the wasm bundle from web/pkg, which lives
      // in the project root (one level up from web/ would be denied otherwise).
      allow: ['..'],
    },
  },
  test: {
    environment: 'jsdom',
    globals: false,
  },
});
