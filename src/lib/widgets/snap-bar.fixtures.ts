// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-27 PR1 SnapBar fixture data + slot type. Lives outside the
// `SnapBar.svelte` component because Svelte's `*.svelte` TS shim only
// re-exports the default export — named exports inside `<script module>`
// are unreachable to consumers. Same workaround as `widgets/index.ts`
// inlines `CuePadCell`.
//
// Wave-36 PR1 adds `kind: SceneKind` per slot so the W36 SceneStill
// canvas-thumbnail widget can replace the W27 PR1 hue-rotating gradient
// surrogate. The kind ↔ scene mapping mirrors the bundle's
// `views/_data.jsx` rows array (e.g. "Quicksand" → `dunes`, "Glob" →
// `fluid`, "Glitch" → `glitch`); operators see the same procedural
// signature per scene name across the SnapBar / Library / Playlist
// surfaces once those views also wire SceneStill.

import type { SceneKind } from './scene-renderers';

/** One quick-access slot. `i` is the 1-indexed slot number rendered
 *  in the lower-left corner of each tile. `tint` is a CSS custom-
 *  property reference (e.g. `var(--color-primary)`) that cascades to
 *  the slot's `--tint` and drives both the active outline and the
 *  slot-number color. `kind` + `seed` parameterise the SceneStill
 *  canvas-renderer per the W36 PR1 wiring. */
export interface SnapSlot {
  readonly i: number;
  readonly name: string;
  readonly scene: string;
  readonly tint: string;
  readonly kind: SceneKind;
  readonly seed: number;
}

/** Bundle-faithful default slots from `extras.jsx` lines 15-23. Bundle
 *  tints (`--cat-color`, `--accent-primary`, `--accent-info`,
 *  `--violet`, `--magenta`, `--orange`, `--accent-danger`,
 *  `--accent-warning`) map onto the closest equivalents in the
 *  Chromaticity token vocabulary. The `kind` per slot mirrors the
 *  bundle's `_data.jsx` rows array scene → procedural-renderer mapping
 *  (e.g. Quicksand uses dunes, Glob uses fluid). */
export const DEFAULT_SNAP_SLOTS: readonly SnapSlot[] = [
  {
    i: 1,
    name: 'Intro',
    scene: 'Quicksand',
    tint: 'var(--color-cat-color)',
    kind: 'dunes',
    seed: 2,
  },
  { i: 2, name: 'Drop 1', scene: 'Glitch', tint: 'var(--color-primary)', kind: 'glitch', seed: 19 },
  { i: 3, name: 'Break', scene: 'Glob', tint: 'var(--color-info)', kind: 'fluid', seed: 14 },
  {
    i: 4,
    name: 'Bass',
    scene: 'Hex Array',
    tint: 'var(--color-signal-violet)',
    kind: 'lattice',
    seed: 11,
  },
  {
    i: 5,
    name: 'Build',
    scene: 'Churning',
    tint: 'var(--color-signal-magenta)',
    kind: 'plasma',
    seed: 15,
  },
  {
    i: 6,
    name: 'Ambient',
    scene: 'Sun_Set',
    tint: 'var(--color-signal-orange)',
    kind: 'rings',
    seed: 5,
  },
  {
    i: 7,
    name: 'Drop 2',
    scene: 'GPU_Melter',
    tint: 'var(--color-danger)',
    kind: 'filaments',
    seed: 22,
  },
  { i: 8, name: 'Outro', scene: 'Ink Shot', tint: 'var(--color-warning)', kind: 'ink', seed: 9 },
];
