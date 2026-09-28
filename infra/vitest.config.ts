import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    environment: 'node',
    // Each variant synthesizes the whole platform (all stacks + cdk-nag).
    testTimeout: 120_000,
    hookTimeout: 300_000,
  },
});
