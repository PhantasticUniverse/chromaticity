// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tauri-aware dev host per https://tauri.app/develop/.
// Empty string / undefined means Vite binds to localhost only.
export default defineConfig(() => {
  const host = process.env['TAURI_DEV_HOST'];

  return {
    plugins: [svelte()],
    clearScreen: false,
    server: {
      port: 1420,
      strictPort: true,
      host: host ?? false,
      ...(host ? { hmr: { protocol: 'ws' as const, host, port: 1421 } } : {}),
      watch: {
        ignored: ['**/src-tauri/**'],
      },
    },
  };
});
