// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { svelte } from '@sveltejs/vite-plugin-svelte';
import fs from 'node:fs';
import path from 'node:path';
import { defineConfig, type Plugin } from 'vite';

/**
 * Restore the load-bearing `dist/.gitkeep` sentinel after `vite build` empties
 * `dist/`. Tauri's `generate_context!()` macro requires `dist/` to exist at
 * `cargo check` time on fresh clones; without this hook every local build
 * leaves a deleted tracked sentinel in `git status`.
 */
function preserveDistGitkeep(): Plugin {
  return {
    name: 'chromaticity-preserve-dist-gitkeep',
    closeBundle() {
      const distDir = path.resolve(process.cwd(), 'dist');
      fs.mkdirSync(distDir, { recursive: true });
      const sentinel = path.join(distDir, '.gitkeep');
      if (!fs.existsSync(sentinel)) {
        fs.writeFileSync(sentinel, '');
      }
    },
  };
}

// Tauri-aware dev host per https://tauri.app/develop/.
// Empty string / undefined means Vite binds to localhost only.
export default defineConfig(() => {
  const host = process.env['TAURI_DEV_HOST'];

  return {
    plugins: [svelte(), preserveDistGitkeep()],
    clearScreen: false,
    server: {
      port: 1420,
      strictPort: true,
      host: host ?? false,
      ...(host ? { hmr: { protocol: 'ws' as const, host, port: 1421 } } : {}),
      watch: {
        ignored: ['**/app/shell/**'],
      },
    },
  };
});
