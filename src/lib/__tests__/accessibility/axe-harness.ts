// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// EV-ACCESSIBILITY-AA T1 helper — Wave 4b.8 PR 1 of 2-3.
//
// Thin wrapper around axe-core scoped to the WCAG 2.2 AA tag set
// (`wcag2a`, `wcag2aa`, `wcag22a`, `wcag22aa`). The helper renders an
// `AccessibilityReport` per call so the cross-view corpus in
// `wcag-aa-corpus.test.ts` can assert against the per-view baseline
// allowlist in `baseline-allowlist.ts`.
//
// `resultTypes: ['violations']` keeps axe's payload focused on the
// failing rules (passes / inapplicable / incomplete are dropped) — the
// per-view budget assertion only cares about the violation count by
// severity, so the trimmed payload keeps test runs fast and the
// failure messages compact.
//
// Wave 4b.8 PR 2 will iterate the per-view allowlist down toward zero
// `critical` / `serious` findings; PR 3 ships the manual VoiceOver /
// NVDA review pass and transitions row 76 → Pass in the registry.

import axe from 'axe-core';

export interface AccessibilityFinding {
  /** axe rule id (e.g. `color-contrast`, `region`). */
  id: string;
  /** Severity per axe; `undefined` only for rules with no impact assigned. */
  impact: 'minor' | 'moderate' | 'serious' | 'critical' | undefined;
  /** Stable docs link for the rule. */
  helpUrl: string;
  /** Number of affected DOM nodes within the audited container. */
  nodes: number;
}

export interface AccessibilityReport {
  /** View name — matches the entry in `baseline-allowlist.ts`. */
  view: string;
  findings: AccessibilityFinding[];
}

/**
 * WCAG 2.2 AA scope is the union of the four axe tag families:
 * - `wcag2a` + `wcag2aa` cover the 2.0 baseline (still load-bearing).
 * - `wcag22a` + `wcag22aa` add the 2.2 deltas.
 *
 * Per axe-core docs, this is the canonical AA-conformance tag union.
 * Tightening to AAA is intentionally out of scope (NFR-11 specifies
 * AA, not AAA).
 */
const WCAG_AA_TAGS = ['wcag2a', 'wcag2aa', 'wcag22a', 'wcag22aa'];

/**
 * Rules disabled at the component-fragment scope. The corpus mounts each
 * routed view in isolation under `@testing-library/svelte`'s default
 * jsdom container — there is no enclosing `<html lang>`, no `<title>`,
 * no `<main>` landmark, no top-level `<h1>`, etc. The rules below only
 * meaningfully apply at the *full-page* scope and report false-positive
 * violations against component-level fragments. axe-core's `rules`
 * parameter is the documented mechanism for this configuration; see
 * https://github.com/dequelabs/axe-core/blob/master/doc/API.md#parameters-axerun
 * (specifically the `rules: { id: { enabled: false }}` shape).
 *
 * This is NOT a relaxation of the WCAG 2.2 AA target. The disabled
 * rules will still apply if a future full-app integration audit lands
 * (e.g. a Playwright-driven `app/shell` boot of the real `index.html`)
 * — that audit would mount the entire shell with `<html lang>` /
 * landmark / heading-one / region scaffolding present, and the same
 * harness function (with the rules array unchanged) would report any
 * page-level structural failures because those failures would no
 * longer be artifacts of the audit-scope mismatch.
 */
const COMPONENT_SCOPE_DISABLED_RULES = [
  'landmark-one-main',
  'landmark-no-duplicate-banner',
  'landmark-no-duplicate-contentinfo',
  'landmark-unique',
  'page-has-heading-one',
  'region',
  'html-has-lang',
  'html-lang-valid',
  'document-title',
  'bypass',
] as const;

/**
 * Run an axe audit against `container` scoped to WCAG 2.2 AA tags.
 *
 * The returned report holds only violations (passes / incomplete /
 * inapplicable are dropped). Per-rule node counts are returned as a
 * scalar so downstream assertions don't need to walk the full axe
 * `NodeResult[]` shape.
 */
export async function runAxeAudit(
  container: HTMLElement,
  view: string,
): Promise<AccessibilityReport> {
  const rules: Record<string, { enabled: false }> = {};
  for (const ruleId of COMPONENT_SCOPE_DISABLED_RULES) {
    rules[ruleId] = { enabled: false };
  }
  const results = await axe.run(container, {
    runOnly: { type: 'tag', values: WCAG_AA_TAGS },
    rules,
    resultTypes: ['violations'],
  });
  return {
    view,
    findings: results.violations.map((v) => ({
      id: v.id,
      impact: v.impact ?? undefined,
      helpUrl: v.helpUrl,
      nodes: v.nodes.length,
    })),
  };
}

/**
 * Filter helper for assertions / debugging — returns the subset of
 * findings at or above a given severity threshold. Order is
 * minor < moderate < serious < critical per axe's documented severity
 * ladder.
 */
export function findingsAtOrAbove(
  report: AccessibilityReport,
  threshold: 'minor' | 'moderate' | 'serious' | 'critical',
): AccessibilityFinding[] {
  const order = { minor: 0, moderate: 1, serious: 2, critical: 3 };
  const min = order[threshold];
  return report.findings.filter((f) => f.impact !== undefined && order[f.impact] >= min);
}
