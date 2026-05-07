// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// EV-ACCESSIBILITY-AA T1 baseline — Wave 4b.8 PR 2 of 2-3.
//
// Per-view axe finding budget. PR 1 captured the current state at
// intentionally-permissive thresholds; PR 2 (this revision) tightens
// every view to a strict zero-tolerance budget after triaging the
// PR-1 findings:
//
//   * Bucket A (component-vs-page scope artifacts) — handled at the
//     harness layer via `COMPONENT_SCOPE_DISABLED_RULES` in
//     `axe-harness.ts` (axe-core's documented `rules` parameter; not
//     a relaxation, since a future full-app integration audit re-
//     enables the page-level rules by definition of its scope).
//
//   * Bucket B (real component-level findings) — fixed in-source.
//     Five findings closed by six small Svelte edits:
//       - VuStereo widget: added `role="img"` so its `aria-label`
//         passes `aria-prohibited-attr`.
//       - CuePad widget: dropped `role="grid"` (which forced a
//         row/cell child shape) and substituted `role="group"` so
//         button children pass `aria-required-children`.
//       - Author view: split the inspector tab strip so the trailing
//         icon-only utility buttons (Reset / Open folder / Export)
//         live in their own `role="toolbar"` group instead of inside
//         the `role="tablist"` — fixes `aria-required-children` and
//         picks up explicit `aria-label`s on the icon buttons.
//       - MIDI view: added `role="img"` to the saved-state dot span
//         so its `aria-label` passes `aria-prohibited-attr`.
//       - Perform view: changed `.rail role="tablist"` →
//         `role="toolbar"` to match the toggle-button (`aria-pressed`)
//         pattern those buttons already use.
//       - CueTransitionCurve widget: changed the SVG container's
//         `role="img"` → `role="group"` so the `role="slider"`
//         handle children no longer trip `nested-interactive`.
//
//   * Bucket C (test-harness artifacts) — none observed.
//
// Wave 4b.8 PR 3 ships the manual VoiceOver / NVDA / keyboard-only
// navigation review pass and transitions row 76 → Pass in the
// registry. Hard-to-automate WCAG 2.2 success criteria (e.g. focus
// order, contrast under live shader output, screen-reader voice
// output) are covered there.
//
// The 14 view names mirror the 14 routed views under
// `src/lib/views/*.svelte`. Adding a new routed view requires adding
// a matching `BASELINE` entry — the corpus's `baseline-allowlist
// covers every routed view` test guards against drift.

export interface BaselineEntry {
  view: string;
  /** Maximum number of `critical`-impact findings tolerated. */
  maxCritical: number;
  /** Maximum number of `serious`-impact findings tolerated. */
  maxSerious: number;
  /** Maximum number of `moderate`-impact findings tolerated. */
  maxModerate: number;
  /** Maximum number of `minor`-impact findings tolerated. */
  maxMinor: number;
}

/**
 * Strict zero-tolerance baseline post-PR-2 remediation. Every routed
 * view audits clean across all four severity tiers under the
 * harness-level component-scope rule disablement (see
 * `axe-harness.ts`). A regression at any tier hard-fails the corpus
 * test — bumping a budget back above zero requires either an
 * accompanying remediation in the same PR or an explicit AO-approved
 * deferral comment naming the rule + view + reason.
 */
export const BASELINE: BaselineEntry[] = [
  { view: 'AudioUniforms', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Author', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Import', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Inputs', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Library', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'License', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'MIDI', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Osc', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Patch', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Perform', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Playlist', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Settings', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Sources', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
  { view: 'Welcome', maxCritical: 0, maxSerious: 0, maxModerate: 0, maxMinor: 0 },
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
