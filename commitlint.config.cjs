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

  // Local plugin carrying the one custom rule
  // `scope-required-for-semver-types`. docs/43 §4.3 requires a scope only
  // for SemVer-bumping types (feat / fix / perf / refactor) — non-SemVer
  // types (docs / test / build / ci / chore / revert) may omit the scope.
  // The built-in `scope-empty` rule is static, so expressing "required
  // for some types only" needs a plugin-registered rule.
  plugins: [
    {
      rules: {
        'scope-required-for-semver-types': ({ type, scope }) => {
          const requiringTypes = new Set(['feat', 'fix', 'perf', 'refactor']);
          if (requiringTypes.has(type) && (!scope || scope.length === 0)) {
            return [
              false,
              `scope is required for type "${type}" per docs/43 §4.3 ` +
                `(mandatory on feat/fix/perf/refactor; optional on ` +
                `docs/test/build/ci/chore/revert)`,
            ];
          }
          return [true];
        },
      },
    },
  ],

  rules: {
    // --- Type ---
    'type-enum': [
      2,
      'always',
      ['feat', 'fix', 'perf', 'refactor', 'docs', 'test', 'build', 'ci', 'chore', 'revert'],
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
    // Scope is conditionally required via the `scope-required-for-semver-types`
    // plugin rule above (mandatory for feat/fix/perf/refactor per docs/43
    // §4.3; optional for all other types). `scope-empty` is not set, which
    // defaults to 0/disabled — an empty scope is permitted by the built-in
    // static rule, and the plugin rule enforces the type-conditional subset.
    'scope-required-for-semver-types': [2, 'always'],

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
