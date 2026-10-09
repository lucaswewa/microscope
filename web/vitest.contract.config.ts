import { defineConfig, mergeConfig } from 'vitest/config'

import viteConfig from './vite.config.ts'

// The contract tests: the WoT client against a running server, whose API
// root is MICROSCOPE_API_URL (`npm run test:contract`; CI's contract job).
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'node',
      include: ['tests/contract/**/*.spec.ts'],
    },
  }),
)
