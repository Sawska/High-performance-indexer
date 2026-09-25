/// <reference types="vitest/config" />
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': process.env.STATUS_URL ?? 'http://localhost:8080',
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['./src/test/setup.ts'],
    // Formatters use the default locale and time zone; pin both so number and
    // date assertions do not depend on the machine running the suite.
    env: { LANG: 'en_US.UTF-8', LC_ALL: 'en_US.UTF-8', TZ: 'UTC' },
    restoreMocks: true,
    unstubGlobals: true,
    // Page tests take well under a second each, but the first one in a file
    // warms up React and jsdom and can pass 5s on a loaded machine or CI runner.
    testTimeout: 20_000,
  },
})
