// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// ESLint flat config per docs/43-development-standards.md §12.2 and
// docs/adr/ADR-018-non-scene-ui-toolkit.md. Rules:
//   @typescript-eslint/recommended
//   eslint-plugin-svelte (flat/recommended; Svelte 5 runes parser)
//   eslint-plugin-import (flat-compatible subset)
//   @typescript-eslint/no-explicit-any: error (§12.2)
//
// ESLint v9+ requires flat config; .eslintrc is not supported.

import js from '@eslint/js';
import importPlugin from 'eslint-plugin-import';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import svelteParser from 'svelte-eslint-parser';
import tseslint from 'typescript-eslint';

export default [
  {
    ignores: [
      // Dependency / build output.
      'node_modules/**',
      'dist/**',
      'target/**',
      '**/target/**',
      'app/shell/target/**',
      'app/shell/gen/**',
      'build/**',
      'out/**',
      '.svelte-kit/**',
      '.vite/**',
      '.local/**',
      // Built MkDocs help corpus (generated HTML/JS/CSS).
      'resources/help/**',
      // Sub-agent worktree state.
      '.claude/**',
      // Clean-room / archive boundaries — never lint these.
      'Contents/**',
      'chromaticity-planning/**',
      // Canonical third-party license texts; preserve upstream.
      'LICENSES/**',
      // Vendored third-party source — preserve upstream formatting.
      'third_party/**',
      // Documentation & user help corpus markdown.
      'docs/**',
      // Local operator-dropped scenes & media (QuickJS scene scripts use runtime globals).
      'scenes-local/**',
      'media-local/**',
    ],
  },

  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],

  {
    plugins: { import: importPlugin },
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: { ...globals.browser },
    },
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      'import/no-unresolved': 'off',
      'import/order': [
        'warn',
        {
          groups: [
            ['builtin', 'external'],
            ['internal', 'parent', 'sibling', 'index'],
          ],
          'newlines-between': 'always',
          alphabetize: { order: 'asc', caseInsensitive: true },
        },
      ],
    },
  },

  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte'],
      },
    },
  },

  // `.svelte.ts` is the Svelte 5 module-context extension for runes
  // outside `.svelte` components (e.g. `src/lib/stores/route.svelte.ts`,
  // a `$state`-backed route singleton). `eslint-plugin-svelte`'s
  // `flat/recommended` claims this glob via `svelte-eslint-parser`,
  // which has no TS-syntax delegate by default, and `class { x = $state(…) }`
  // (a TS class field with rune RHS) parses as an unexpected token.
  // The override re-routes `.svelte.ts` through the typescript-eslint
  // parser so class fields, runes, and TS syntax all parse cleanly.
  // See `CLAUDE.md` "eslint-plugin-svelte claims .svelte.ts" feedback.
  {
    files: ['**/*.svelte.ts'],
    languageOptions: {
      parser: tseslint.parser,
    },
  },

  {
    files: [
      '**/*.config.{js,cjs,mjs,ts}',
      'vite.config.ts',
      'svelte.config.js',
      'commitlint.config.cjs',
    ],
    languageOptions: {
      globals: { ...globals.node },
    },
  },
];
