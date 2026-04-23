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
      'src-tauri/target/**',
      'src-tauri/gen/**',
      'build/**',
      'out/**',
      'generated/**',
      '.svelte-kit/**',
      '.vite/**',
      // Clean-room / archive boundaries — never lint these.
      // See docs/29-cleanroom-legal-policy-baseline.md §Clean-room SOP.
      'Contents/**',
      'chromaticity-planning/**',
      // Canonical third-party license texts; preserve upstream.
      'LICENSES/**',
      // Grandfathered planning prose (promoted normative docs).
      'docs/**',
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
