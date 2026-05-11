// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.2 Lane R.1 — Scene-metadata → SceneKind mapping helper. The
// 16 canvas-2d procedural renderers in `./scene-renderers.ts` are
// addressed by name (`SceneKind`); when wiring `<SceneCanvas>` /
// `<SceneStill>` into the Perform preview hero / Library scene-card
// grid / Playlist cue-row thumbnails / Author live-preview tile, the
// view layer must pick a kind for each scene. The wireframe-canonical
// pattern is the `SnapBar` + setlist fixture per-row kind (see
// `snap-bar.fixtures.ts` lines 42-86 and `setlist-fixtures.ts` lines
// 48-59) — Quicksand → dunes, Glob → fluid, Glitch → glitch, etc.
// This helper extends that pattern to live `.chroma` scenes loaded
// through `loadedSceneStore` and `SceneCard` rows discovered via the
// library indexer; no engine surface widens — `scene.json` does NOT
// carry a `KIND` field today and the heuristic operates purely on the
// existing TITLE / DESCRIPTION / TAGS / CATEGORIES metadata.
//
// **Heuristic priority** (first match wins):
//   1. Exact title token match against the 12 canonical bundle scenes.
//   2. Keyword substring match across `title + description + tags +
//      categories` (lowercased, joined with spaces).
//   3. Category fallback (`audio reactive` → bars, `geometric` →
//      lattice, etc).
//   4. Default `rings`.
//
// Goal per the AO brief: each `.chroma` scene gets a visually-
// distinct preview that feels intentional — `library/demo-kaleido`
// looks kaleido, scenes tagged `audio reactive` show bars, geometric
// scenes show lattice / grid.

import { SCENE_KINDS, type SceneKind } from './scene-renderers';

/**
 * Minimal scene-metadata shape this helper consumes. Designed so both
 * `SceneCard` (the IPC-facing library row) and `LoadedScenePayload`
 * (the active-scene wire-shape) can satisfy it via field-name lift
 * without a runtime conversion step.
 */
export interface SceneMetadataLike {
  readonly id?: string | null;
  readonly title?: string | null;
  readonly name?: string | null;
  readonly description?: string | null;
  readonly tags?: readonly string[] | null;
  readonly categories?: readonly string[] | null;
}

/**
 * Bundle-canonical scene-title → SceneKind table, mirroring the
 * `setlist-fixtures.ts` 12-row corpus and the `snap-bar.fixtures.ts`
 * defaults. Keys are lower-cased so operator typing in TITLE matches
 * regardless of case. Trailing punctuation / digits in the loaded
 * scene's title fall through to the keyword pass.
 */
const TITLE_EXACT: Readonly<Record<string, SceneKind>> = {
  quicksand: 'dunes',
  glob: 'fluid',
  sun_set: 'rings',
  sunset: 'rings',
  glitch: 'glitch',
  glassier: 'kaleido',
  'ink shot': 'ink',
  'hex array': 'lattice',
  squareshoals: 'grid',
  churning: 'plasma',
  myoplasm: 'cells',
  myxoplasm: 'cells',
  gpu_melter: 'filaments',
  'test card': 'bars',
};

/**
 * Keyword → kind table for the substring pass. Order matters only
 * loosely (Object.entries preserves insertion order); the first match
 * wins. Wider terms come after narrower ones so e.g. `kaleidoscope`
 * resolves before `scope`. Each key is checked as a lower-case
 * substring against the joined haystack.
 */
const KEYWORD_KIND: ReadonlyArray<readonly [string, SceneKind]> = [
  ['kaleidoscope', 'kaleido'],
  ['kaleido', 'kaleido'],
  ['tunnel', 'tunnel'],
  ['plasma', 'plasma'],
  ['glitch', 'glitch'],
  ['voronoi', 'cells'],
  ['cells', 'cells'],
  ['filament', 'filaments'],
  ['wire', 'filaments'],
  ['thread', 'filaments'],
  ['lattice', 'lattice'],
  ['hex', 'lattice'],
  ['grid', 'grid'],
  ['rain', 'rain'],
  ['matrix', 'rain'],
  ['orbit', 'orbit'],
  ['particle', 'orbit'],
  ['noise', 'noise'],
  ['ink', 'ink'],
  ['fluid', 'fluid'],
  ['glob', 'fluid'],
  ['blob', 'fluid'],
  ['dune', 'dunes'],
  ['sand', 'dunes'],
  ['wave', 'dunes'],
  ['ring', 'rings'],
  ['spectrum', 'bars'],
  ['fft', 'bars'],
  ['bar', 'bars'],
  ['vu', 'bars'],
  ['beat', 'bars'],
  ['bpm', 'bars'],
];

/**
 * Category fallback table. Categories are upper-case in `SceneCard`
 * per the Library chip vocabulary; the lookup lowercases them.
 */
const CATEGORY_KIND: Readonly<Record<string, SceneKind>> = {
  'audio reactive': 'bars',
  geometric: 'lattice',
  '3d': 'tunnel',
  text: 'glitch',
  organic: 'fluid',
  abstract: 'rings',
  glitch: 'glitch',
  'built-in': 'rings',
};

/**
 * Resolve a `SceneKind` from a scene's metadata via the priority
 * pipeline documented in the module header. Returns `rings` when no
 * heuristic fires.
 */
export function sceneKindFromMetadata(meta: SceneMetadataLike | null | undefined): SceneKind {
  if (!meta) return 'rings';

  const title = (meta.title ?? meta.name ?? '').trim().toLowerCase();

  // 1) Exact title match.
  if (title && TITLE_EXACT[title] !== undefined) {
    return TITLE_EXACT[title];
  }

  // 2) Title sometimes already names a kind directly (e.g. a scene
  //    titled "Kaleido" or a demo `library/demo-kaleido`). Check
  //    SCENE_KINDS first for that fast path before the keyword sweep.
  for (const k of SCENE_KINDS) {
    if (title.includes(k)) return k;
  }

  // 3) Keyword sweep against the joined haystack.
  const haystack = [
    title,
    (meta.description ?? '').toLowerCase(),
    ...(meta.tags ?? []).map((t) => t.toLowerCase()),
    ...(meta.categories ?? []).map((c) => c.toLowerCase()),
  ].join(' ');

  for (const [needle, kind] of KEYWORD_KIND) {
    if (haystack.includes(needle)) return kind;
  }

  // 4) Category fallback.
  for (const cat of meta.categories ?? []) {
    const key = cat.trim().toLowerCase();
    if (CATEGORY_KIND[key] !== undefined) return CATEGORY_KIND[key];
  }

  return 'rings';
}

/**
 * Deterministic 32-bit seed derived from a scene's identifier. FNV-1a
 * hash of `id ?? title ?? name`, then folded to a small range so the
 * renderer's `seed * 13.7` / `seed * 40` multiplications produce
 * visually-distinct phases without colour-wheel saturation.
 *
 * Returns `0` when no usable identifier is present (matches the
 * `seed = 0` default on `<SceneStill>` / `<SceneCanvas>`).
 */
export function sceneSeedFromMetadata(meta: SceneMetadataLike | null | undefined): number {
  if (!meta) return 0;
  const key = (meta.id ?? meta.title ?? meta.name ?? '').toString();
  if (!key) return 0;
  let hash = 0x811c9dc5; // FNV-1a 32-bit offset basis.
  for (let i = 0; i < key.length; i++) {
    hash ^= key.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0; // FNV prime, unsigned.
  }
  // Fold to 0..63 — keeps trig phases readable while still varied.
  return hash % 64;
}
