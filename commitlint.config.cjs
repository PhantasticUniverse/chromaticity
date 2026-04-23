// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Commitlint configuration.
// Enforces docs/43-development-standards.md Sec.4 on:
//   - Feature-branch commits (advisory, via lefthook commit-msg hook).
//   - The PR squash-merge commit that lands on `main` (mandatory, via CI).
//
// Adjustments here require a `policy`-scoped PR editing Sec.4.2/4.3.

/** @type {import('@commitlint/types').UserConfig} */
module.exports = {
  extends: ['@commitlint/config-conventional'],

  rules: {
    // --- Type ---
    'type-enum': [
      2,
      'always',
      [
        'feat',
        'fix',
        'perf',
        'refactor',
        'docs',
        'test',
        'build',
        'ci',
        'chore',
        'revert',
      ],
    ],
    'type-case': [2, 'always', 'lower-case'],
    'type-empty': [2, 'never'],

    // --- Scope (vocabulary fixed; additions require a `policy` commit) ---
    'scope-enum': [
      2,
      'always',
      [
        'runtime',
        'scene',
        'audio',
        'integration',
        'authoring',
        'library',
        'ui',
        'docs',
        'ux',
        'entitlement',
        'observability',
        'release-eng',
        'policy',
        'repo',
      ],
    ],
    'scope-case': [2, 'always', 'lower-case'],
    // Scope is mandatory for feat/fix/perf/refactor per Sec.4.3.
    // commitlint does not have a conditional "empty only for some types"
    // rule natively; `scope-empty` = 2/never is applied to all and we
    // accept the mild over-enforcement (all real Chromaticity commits
    // carry a scope).
    'scope-empty': [2, 'never'],

    // --- Subject ---
    'subject-case': [2, 'always', 'lower-case'],
    'subject-empty': [2, 'never'],
    'subject-full-stop': [2, 'never', '.'],
    'subject-max-length': [2, 'always', 72],

    // --- Body / footer wrap ---
    'body-max-line-length': [2, 'always', 100],
    'footer-max-line-length': [2, 'always', 100],
    'body-leading-blank': [2, 'always'],
    'footer-leading-blank': [2, 'always'],

    // --- Header overall cap ---
    'header-max-length': [2, 'always', 100],
  },
};
