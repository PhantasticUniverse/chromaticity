// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Vitest config — first frontend test framework in repo history. Kept
// deliberately minimal: jsdom environment for DOM assertions on Svelte
// primitives, scoped include glob to `src/**/*.test.ts`, and the
// `browser` resolve condition so Svelte's browser entry resolves under
// jsdom (otherwise the SSR build kicks in and component renders fail).

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte({ hot: !process.env['VITEST'] })],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    setupFiles: ['src/lib/test-setup.ts'],
    globals: false,
  },
  resolve: {
    conditions: ['browser'],
  },
});
