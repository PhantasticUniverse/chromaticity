// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-13 PR2: pure math module backing `CueTransitionCurve.svelte`. Lives
// alongside the component (rather than inside the .svelte module) so the
// cubic-bezier evaluator and the preset reverse-lookup can be exercised by
// Vitest without spinning up a renderer. The component imports from here;
// no DOM references and no Svelte runes in this file.
//
// Source of truth: `chromaticity-planning/ui-predev-bundle-v0.2-beta/cue_curve.jsx`
// (`bezier1d` and `presetFor` helpers) per the Wave-13 PR2 port plan and
// the wireframes-normative memory note. Numeric values match the bundle
// to four-decimal precision.

export type CueKind = 'cut' | 'fade' | 'snap';
export type CueSnap = '0.25' | '0.5' | '1' | '2' | '4';
export type CuePoint = readonly [number, number];

export interface CuePreset {
  readonly id: string;
  readonly label: string;
  readonly cp1: CuePoint;
  readonly cp2: CuePoint;
}

/**
 * Preset catalogue. Each preset names two cubic-bezier control points in
 * the unit square; `p0` is always (0, 0) and `p3` always (1, 1).
 */
export const CUE_PRESETS: readonly CuePreset[] = [
  { id: 'linear', label: 'Linear', cp1: [0.0, 0.0], cp2: [1.0, 1.0] },
  { id: 'in', label: 'Ease In', cp1: [0.42, 0.0], cp2: [1.0, 1.0] },
  { id: 'out', label: 'Ease Out', cp1: [0.0, 0.0], cp2: [0.58, 1.0] },
  { id: 'inout', label: 'In·Out', cp1: [0.42, 0.0], cp2: [0.58, 1.0] },
  { id: 'expo', label: 'Expo', cp1: [0.87, 0.0], cp2: [0.13, 1.0] },
  { id: 'scurve', label: 'S-Curve', cp1: [0.65, 0.04], cp2: [0.35, 0.96] },
];

/**
 * Cubic-bezier evaluation: standard `B(t) = (1-t)^3·p0 + 3(1-t)^2·t·p1 + 3(1-t)·t^2·p2 + t^3·p3`.
 * Used for both x-axis and y-axis projections of the ghost-dot animation.
 */
export function bezier1d(t: number, p0: number, p1: number, p2: number, p3: number): number {
  const mt = 1 - t;
  return mt * mt * mt * p0 + 3 * mt * mt * t * p1 + 3 * mt * t * t * p2 + t * t * t * p3;
}

/**
 * Reverse-lookup against `CUE_PRESETS`. Compares to two-decimal precision
 * (matches the bundle's `Math.round(n * 100) / 100`); returns the matching
 * preset or `undefined` for arbitrary control points (i.e. "Custom").
 */
export function presetFor(cp1: CuePoint, cp2: CuePoint): CuePreset | undefined {
  const round = (n: number): number => Math.round(n * 100) / 100;
  const match = (a: CuePoint, b: CuePoint): boolean =>
    round(a[0]) === round(b[0]) && round(a[1]) === round(b[1]);
  return CUE_PRESETS.find((p) => match(p.cp1, cp1) && match(p.cp2, cp2));
}

/**
 * Clamp a number to `[lo, hi]`. Used by the drag handler when projecting
 * pointer coordinates back onto the unit square; kept here so the same
 * clamp logic is observable via Vitest if it ever needs spot-checking.
 */
export function clamp01(n: number): number {
  if (n < 0) return 0;
  if (n > 1) return 1;
  return n;
}
