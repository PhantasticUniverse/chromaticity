/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * Setlist fixtures — TypeScript mirror of the v0.2 bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/_data.jsx`
 * `SCENES` constant, used by the Library view's right-sidebar setlist
 * panel per the v0.2 wireframe-01 chrome.
 *
 * The bundle JSX renders 14 setlist rows by `SCENES.concat(SCENES).slice(0, 14)`
 * — i.e. the 12-entry SCENES array repeated to fill 14 cue slots, with
 * the row at index 3 ("Glitch") highlighted as active. This module
 * mirrors that exact pattern so the production setlist visually
 * matches the wireframe.
 *
 * W15 PR3 posture: hardcoded fixtures only. Real setlist persistence
 * + cue-edit IPC is Phase-3 `EV-PRESET-PLAYLIST` scope; the bundle
 * JSX uses a similar fixture pattern (`active={i === 3}`) so the
 * production-side chrome can land before the runtime cue store.
 *
 * Per the `feedback_wireframes_normative_for_ui_ux.md` memory, the
 * bundle JSX is the canonical source; this fixture mirrors the JSX
 * verbatim so future updates sync cleanly through both surfaces.
 */

export type SetlistTag = 'default' | 'hot' | 'caution';

export interface SetlistSceneFixture {
  /** Scene name as displayed in the sidebar row. */
  readonly name: string;
  /** Tag chip (default | hot | caution). */
  readonly tag: SetlistTag;
  /** Display duration string (e.g. "00h 02m 00s"). */
  readonly duration: string;
  /** Per-row variation seed — drives placeholder thumbnail tinting. */
  readonly seed: number;
  /** Render kind — informational; placeholder thumbnails today. */
  readonly kind: string;
}

/**
 * 12-row scene base fixture mirroring the bundle's `SCENES`
 * (`views/_data.jsx` lines 5-18). The Library setlist sidebar
 * concatenates this twice and slices to 14 rows per the v0.2 JSX
 * pattern.
 */
export const SETLIST_SCENES: readonly SetlistSceneFixture[] = [
  { name: 'Quicksand', tag: 'default', duration: '00h 02m 00s', seed: 2, kind: 'dunes' },
  { name: 'Glob', tag: 'default', duration: '00h 02m 00s', seed: 14, kind: 'fluid' },
  { name: 'Sun_Set', tag: 'default', duration: '00h 02m 00s', seed: 5, kind: 'rings' },
  { name: 'Glitch', tag: 'hot', duration: '00h 02m 00s', seed: 19, kind: 'glitch' },
  { name: 'Glassier', tag: 'default', duration: '00h 02m 00s', seed: 7, kind: 'kaleido' },
  { name: 'Ink Shot', tag: 'default', duration: '00h 02m 00s', seed: 9, kind: 'ink' },
  { name: 'Hex Array', tag: 'default', duration: '00h 02m 00s', seed: 11, kind: 'lattice' },
  { name: 'SquareShoals', tag: 'default', duration: '00h 02m 00s', seed: 3, kind: 'grid' },
  { name: 'Churning', tag: 'default', duration: '00h 02m 00s', seed: 15, kind: 'plasma' },
  { name: 'Myoplasm', tag: 'default', duration: '00h 02m 00s', seed: 18, kind: 'cells' },
  { name: 'GPU_Melter', tag: 'caution', duration: '00h 02m 00s', seed: 22, kind: 'filaments' },
  { name: 'Test Card', tag: 'default', duration: '00h 02m 00s', seed: 1, kind: 'bars' },
];

/**
 * 14-row setlist matching the bundle's `SCENES.concat(SCENES).slice(0, 14)`
 * pattern. Indexed (0-based) for stable row identity in the sidebar.
 */
export const SETLIST_FIXTURE: ReadonlyArray<SetlistSceneFixture & { idx: number }> =
  SETLIST_SCENES.map((scene, idx) => ({ ...scene, idx }));

/**
 * Index of the row rendered as "active" (currently-loaded) in W15 PR3.
 * Matches the bundle JSX `active={i === 3}` pattern. Phase-3 wires
 * this to the live runtime scene-load state.
 */
export const SETLIST_ACTIVE_INDEX = 3;
