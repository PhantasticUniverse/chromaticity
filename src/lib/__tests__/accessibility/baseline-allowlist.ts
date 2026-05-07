// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// EV-ACCESSIBILITY-AA T1 baseline — Wave 4b.8 PR 1 of 2-3.
//
// Per-view axe finding budget. PR 1 captures the current state at
// intentionally-permissive thresholds so the harness file
// (`wcag-aa-corpus.test.ts`) does not red-line CI on first land —
// every routed view is in scope, the budget is read-only here, and
// the corpus enforces `findings <= budget` per severity.
//
// Wave 4b.8 PR 2 will iterate the per-view findings down toward zero
// `critical` / `serious` findings and tighten the budgets here as the
// remediation lands. Wave 4b.8 PR 3 ships the manual VoiceOver / NVDA
// review pass and transitions row 76 → Pass in the registry.
//
// The 14 view names mirror the 14 routed views under
// `src/lib/views/*.svelte`. Adding a new routed view requires adding
// a matching `BASELINE` entry — the corpus's `baseline-allowlist
// covers every routed view` test guards against drift.

export interface BaselineEntry {
  view: string;
  /** Maximum number of `critical`-impact findings tolerated for PR 1. */
  maxCritical: number;
  /** Maximum number of `serious`-impact findings tolerated for PR 1. */
  maxSerious: number;
  /** Maximum number of `moderate`-impact findings tolerated for PR 1. */
  maxModerate: number;
  /** Maximum number of `minor`-impact findings tolerated for PR 1. */
  maxMinor: number;
}

/**
 * Permissive PR-1 baseline. PR 2 tightens budgets per view as
 * findings drop; the harness assertions are `<=`, so tightening is
 * a one-line edit per row once a view's findings shrink.
 */
export const BASELINE: BaselineEntry[] = [
  { view: 'AudioUniforms', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Author', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Import', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Inputs', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Library', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'License', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'MIDI', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Osc', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Patch', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Perform', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Playlist', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Settings', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Sources', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
  { view: 'Welcome', maxCritical: 5, maxSerious: 10, maxModerate: 20, maxMinor: 50 },
];

/**
 * Look up the budget entry for `view`. Throws if missing — the
 * cross-view corpus's drift guard test will catch the mismatch as
 * well, but throwing here keeps the failure proximate to the
 * offending render call.
 */
export function baselineFor(view: string): BaselineEntry {
  const entry = BASELINE.find((b) => b.view === view);
  if (!entry) {
    throw new Error(`No baseline entry for view: ${view}. Add one to baseline-allowlist.ts.`);
  }
  return entry;
}
