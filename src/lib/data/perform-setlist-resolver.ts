/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * Perform-view setlist resolver — maps a `SETLIST_FIXTURE` row title
 * (e.g. "Glitch", "Glob", "Test Card") to a real `.chroma` package
 * identifier under `engine/scene/tests/fixtures/scenes/`. Used by the
 * Perform right-column scene list (Wave 2.4 Lane PERF.B) to wire
 * row-click into `loadedSceneStore.load(card.path)`.
 *
 * **Why a fixture-to-corpus mapper exists.** `SETLIST_FIXTURE` mirrors
 * the v0.2 bundle's `views/_data.jsx` `SCENES` 12-row corpus verbatim
 * per the wireframes-normative convention (see
 * `feedback_wireframes_normative_for_ui_ux.md`). The bundle's row
 * titles ("Quicksand", "Glob", "Sun_Set", …) do not match the
 * Wave 1d-β clean-room scene corpus on disk (`demo-kaleido`,
 * `welcome`, `audio-reactive-demo`, …) — the fixture was a design
 * preview before the corpus existed. This resolver bridges the two so
 * the setlist remains wireframe-faithful for chrome while clicks load
 * a real scene from the on-disk corpus.
 *
 * **Mapping strategy.** Heuristic priority (first match wins), modelled
 * after `widgets/scene-kind-from-metadata.ts`:
 *   1. Exact title-token match (lower-case, punctuation-stripped)
 *      against the 12 canonical bundle scenes, hand-picked to the
 *      closest available `.chroma` package by visual / behavioural kin.
 *   2. Keyword substring against the row's `name` and `kind` fields.
 *   3. Return `null` — the row stays display-only (no click handler).
 *
 * The resolver returns a scene **id** (the `.chroma` directory stem,
 * e.g. `demo-kaleido`); the caller resolves the absolute filesystem
 * path via `fetchBundledSceneCards()` lookup since paths depend on
 * runtime context (Tauri's `<AppBundle>/Resources/scenes/...` resolves
 * differently per OS and is unknown to this pure mapper).
 *
 * **Coverage.** Of the 12 bundle setlist titles, 4 resolve to demo
 * scenes by closest-kind match (`Glassier`/kaleido → `demo-kaleido`,
 * `Test Card`/bars → `demo-spectrum`, `Glitch`/glitch → `demo-text`
 * stand-in, `Sun_Set` → `welcome`). The remaining 8 return `null` and
 * stay display-only — clicking does nothing — pending a future-wave
 * corpus widening that closes the gap.
 */

import type { SetlistSceneFixture } from './setlist-fixtures';

/**
 * Hand-picked title-token → `.chroma` id map. Keys are lower-cased,
 * spaces / underscores collapsed. The mapped id is the `.chroma`
 * directory stem as returned by the engine's `LoadedScenePayload.id`
 * (see `app/shell/src/commands/scene.rs::build_loaded_scene_payload`).
 *
 * Pairings reflect closest-kind heuristic:
 *   - kaleidoscopic / glassy → `demo-kaleido`
 *   - text overlay / glitchy → `demo-text`
 *   - audio bars / test pattern → `demo-spectrum`
 *   - default / first-cue → `welcome`
 */
const TITLE_TO_ID: Readonly<Record<string, string>> = {
  glassier: 'demo-kaleido',
  glitch: 'demo-text',
  'test card': 'demo-spectrum',
  testcard: 'demo-spectrum',
  sun_set: 'welcome',
  sunset: 'welcome',
};

/**
 * Setlist row → `.chroma` id resolver. Returns `null` when no scene in
 * the on-disk corpus matches the row's title — the caller should drop
 * the click handler so the row stays display-only.
 *
 * The input shape is intentionally narrowed to the two fields the
 * resolver consults (`name`, `kind`); callers may pass a full
 * `SETLIST_FIXTURE` entry or a synthetic stub from tests.
 */
export function resolveSetlistRowToSceneId(
  row: Pick<SetlistSceneFixture, 'name' | 'kind'>,
): string | null {
  const titleKey = row.name.trim().toLowerCase().replace(/\s+/g, ' ');
  if (TITLE_TO_ID[titleKey] !== undefined) {
    return TITLE_TO_ID[titleKey];
  }
  // Normalised form without spaces — handles "Test Card" → "testcard".
  const compactKey = titleKey.replace(/\s+/g, '');
  if (TITLE_TO_ID[compactKey] !== undefined) {
    return TITLE_TO_ID[compactKey];
  }

  // Kind-keyword fallback. The setlist `kind` field is the bundle's
  // free-form renderer hint ("dunes" / "fluid" / "bars" / …). When a
  // row's title is not in the curated table, an audio-bars-kind row
  // can still load the audio spectrum demo.
  const kindKey = row.kind.trim().toLowerCase();
  switch (kindKey) {
    case 'bars':
      return 'demo-spectrum';
    case 'kaleido':
      return 'demo-kaleido';
    case 'glitch':
      return 'demo-text';
    default:
      return null;
  }
}
