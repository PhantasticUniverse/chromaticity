/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Scene-library helper module for the W9 backbone slice.
//
// Plain `.ts` (not `.svelte.ts`) so eslint-plugin-svelte's parser does
// not claim the file. Reactive state lives in the consuming component
// via `$state`; this module exposes the IPC sequencing plus a per-root
// in-memory cache so successive scans of the same path do not re-cross
// the IPC boundary.
//
// W11 PR2 adds `fetchBundledSceneCards` for the ship-time corpus
// surfaced via `list_bundled_scene_packages` IPC; the bundled cards
// share the same per-root cache map keyed on a sentinel string.

import type { SceneCard } from '../ipc/bindings';
import { defaultSceneRoot, listBundledScenePackages, listScenePackages } from '../ipc/library';

const cache = new Map<string, SceneCard[]>();
const BUNDLED_CACHE_KEY = '__bundled__';

/// Read the platform-default scene-library root path.
export async function getDefaultRoot(): Promise<string> {
  return defaultSceneRoot();
}

/// Fetch the `.chroma` packages directly under `root`, returning a
/// cached result on a repeat call for the same `root`.
export async function fetchSceneCards(root: string): Promise<SceneCard[]> {
  const cached = cache.get(root);
  if (cached) return cached;
  const cards = await listScenePackages(root);
  cache.set(root, cards);
  return cards;
}

/// Fetch the `.chroma` packages bundled into the Tauri AppBundle's
/// `Resources/scenes/` directory at link time. Cards are tagged
/// `source: "Bundled"` server-side (W11 PR2).
export async function fetchBundledSceneCards(): Promise<SceneCard[]> {
  const cached = cache.get(BUNDLED_CACHE_KEY);
  if (cached) return cached;
  const cards = await listBundledScenePackages();
  cache.set(BUNDLED_CACHE_KEY, cards);
  return cards;
}

/// Invalidate the per-root cache. Callers invoke this on rescan-on-
/// user-request and on filesystem-change events per
/// `docs/38-scene-library-and-import-spec.md` §2.1.
export function clearCache(): void {
  cache.clear();
}
