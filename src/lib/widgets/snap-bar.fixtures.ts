// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave-27 PR1 SnapBar fixture data + slot type. Lives outside the
// `SnapBar.svelte` component because Svelte's `*.svelte` TS shim only
// re-exports the default export — named exports inside `<script module>`
// are unreachable to consumers. Same workaround as `widgets/index.ts`
// inlines `CuePadCell`.

/** One quick-access slot. `i` is the 1-indexed slot number rendered
 *  in the lower-left corner of each tile. `tint` is a CSS custom-
 *  property reference (e.g. `var(--color-primary)`) that cascades to
 *  the slot's `--tint` and drives both the active outline and the
 *  slot-number color. */
export interface SnapSlot {
  readonly i: number;
  readonly name: string;
  readonly scene: string;
  readonly tint: string;
  readonly seed?: number;
}

/** Bundle-faithful default slots from `extras.jsx` lines 15-23. Bundle
 *  tints (`--cat-color`, `--accent-primary`, `--accent-info`,
 *  `--violet`, `--magenta`, `--orange`, `--accent-danger`,
 *  `--accent-warning`) map onto the closest equivalents in the
 *  Chromaticity token vocabulary. */
export const DEFAULT_SNAP_SLOTS: readonly SnapSlot[] = [
  { i: 1, name: 'Intro', scene: 'Quicksand', tint: 'var(--color-cat-color)', seed: 2 },
  { i: 2, name: 'Drop 1', scene: 'Glitch', tint: 'var(--color-primary)', seed: 19 },
  { i: 3, name: 'Break', scene: 'Glob', tint: 'var(--color-info)', seed: 14 },
  { i: 4, name: 'Bass', scene: 'Hex Array', tint: 'var(--color-signal-violet)', seed: 11 },
  { i: 5, name: 'Build', scene: 'Churning', tint: 'var(--color-signal-magenta)', seed: 15 },
  { i: 6, name: 'Ambient', scene: 'Sun_Set', tint: 'var(--color-signal-orange)', seed: 5 },
  { i: 7, name: 'Drop 2', scene: 'GPU_Melter', tint: 'var(--color-danger)', seed: 22 },
  { i: 8, name: 'Outro', scene: 'Ink Shot', tint: 'var(--color-warning)', seed: 9 },
];
