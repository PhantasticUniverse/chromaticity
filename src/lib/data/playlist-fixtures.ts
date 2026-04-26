/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Wave-13 PR3 fixture data for the Playlist v2 view.
 *
 * Source of truth (port reference):
 *   chromaticity-planning/ui-predev-bundle-v0.2-beta/views/playlist.jsx
 * Wireframe target:
 *   .playwright-mcp/wireframe-reference/03-playlist-cue-table.png
 *
 * Hardcoded fixture per the W10 Inputs precedent: cue persistence is
 * Phase-3 EV-PRESET-PLAYLIST scope and out of band for W13. The shape
 * here is deliberately structured to be close to the future IPC payload
 * so a W14+ persistence pass can substitute the data source without
 * reshaping the view's render.
 *
 * The wireframe-alignment posture (per the project memory note
 * `feedback_wireframes_normative_for_ui_ux`) keeps the visual layout +
 * data-shape bonded; deviations from the wireframe `.png` capture must
 * be deliberate, not incidental.
 *
 * Only the active "Festival Set / Dusk" playlist carries a cue list.
 * The four other playlists (Ambient Pool, Hard Techno · 140, Sound Check,
 * Demo Scenes) intentionally render the empty-state placeholder until
 * W14+ extends the fixture set or wires real persistence.
 */

export interface PlaylistFixture {
  readonly id: string;
  readonly name: string;
  readonly active?: boolean;
}

/**
 * Cue transition family. `'beat-snap'` is the inspector-side fourth tab in
 * the wireframe; the underlying `CueTransitionCurve` widget collapses it
 * onto its `'snap'` kind for the curve render. The view-level tab strip
 * preserves the 4-way distinction so operator muscle memory matches the
 * mockup, even though the curve renderer treats 'snap' and 'beat-snap'
 * identically.
 */
export type CueTransitionKind = 'cut' | 'fade' | 'snap' | 'beat-snap';

/** Snap-divisor enum mirrored from `cue-transition-curve.math.ts::CueSnap`. */
export type CueSnapDivisor = '0.25' | '0.5' | '1' | '2' | '4';

export interface CueFixture {
  readonly index: number;
  readonly name: string;
  readonly scene: string;
  readonly duration: string;
  readonly crossfade: string;
  readonly transition: CueTransitionKind;
  readonly cp1: readonly [number, number];
  readonly cp2: readonly [number, number];
  readonly snap: CueSnapDivisor;
  readonly controlPreset: string;
}

export const PLAYLISTS: readonly PlaylistFixture[] = [
  { id: 'festival-set-dusk', name: 'Festival Set / Dusk', active: true },
  { id: 'ambient-pool', name: 'Ambient Pool' },
  { id: 'hard-techno-140', name: 'Hard Techno · 140' },
  { id: 'sound-check', name: 'Sound Check' },
  { id: 'demo-scenes', name: 'Demo Scenes' },
];

/**
 * 10-row Festival Set / Dusk cue list. Names + scenes mirror the wireframe
 * row labels (the bundle's `playlist.jsx` `rows` array, with durations and
 * crossfade strings normalised against the wireframe capture). The control
 * preset column tracks the wireframe's PRESET pill text — most rows fall
 * under "default", with the techno block (rows 3-5, 7) labelled
 * "Hard Techno" and the closing cue tagged "Ambient" per the mockup.
 */
export const FESTIVAL_SET_CUES: readonly CueFixture[] = [
  {
    index: 1,
    name: 'Intro Drift',
    scene: 'Quicksand',
    duration: '02:30',
    crossfade: '06.4s',
    transition: 'fade',
    cp1: [0.42, 0],
    cp2: [0.58, 1],
    snap: '1',
    controlPreset: 'default',
  },
  {
    index: 2,
    name: 'Boring',
    scene: 'Glob',
    duration: '02:00',
    crossfade: '06.4s',
    transition: 'fade',
    cp1: [0.42, 0],
    cp2: [0.58, 1],
    snap: '1',
    controlPreset: 'default',
  },
  {
    index: 3,
    name: 'First Drop',
    scene: 'Glitch',
    duration: '03:30',
    crossfade: '04.8s',
    transition: 'fade',
    cp1: [0.7, 0],
    cp2: [0.3, 1],
    snap: '1',
    controlPreset: 'Hard Techno',
  },
  {
    index: 4,
    name: 'Peak',
    scene: 'GPU_Melter',
    duration: '02:30',
    crossfade: '04.8s',
    transition: 'snap',
    cp1: [0.5, 0],
    cp2: [0.5, 1],
    snap: '0.25',
    controlPreset: 'Hard Techno',
  },
  {
    index: 5,
    name: 'Release',
    scene: 'Glassier',
    duration: '03:30',
    crossfade: '04.8s',
    transition: 'fade',
    cp1: [0.42, 0],
    cp2: [0.58, 1],
    snap: '1',
    controlPreset: 'Hard Techno',
  },
  {
    index: 6,
    name: 'Plateau',
    scene: 'Churning',
    duration: '04:30',
    crossfade: '04.8s',
    transition: 'fade',
    cp1: [0.42, 0],
    cp2: [0.58, 1],
    snap: '1',
    controlPreset: 'default',
  },
  {
    index: 7,
    name: 'Callback',
    scene: 'Hex Array',
    duration: '03:30',
    crossfade: '04.8s',
    transition: 'snap',
    cp1: [0.5, 0],
    cp2: [0.5, 1],
    snap: '0.5',
    controlPreset: 'Hard Techno',
  },
  {
    index: 8,
    name: 'Ink Slam',
    scene: 'Ink Shot',
    duration: '02:30',
    crossfade: '04.8s',
    transition: 'cut',
    cp1: [0, 0],
    cp2: [1, 1],
    snap: '1',
    controlPreset: 'default',
  },
  {
    index: 9,
    name: 'Second Drop',
    scene: 'MyxoplASM',
    duration: '04:30',
    crossfade: '04.8s',
    transition: 'fade',
    cp1: [0.7, 0],
    cp2: [0.3, 1],
    snap: '1',
    controlPreset: 'default',
  },
  {
    index: 10,
    name: 'Cooldown',
    scene: 'Sun_Set',
    duration: '02:00',
    crossfade: '06.4s',
    transition: 'fade',
    cp1: [0.42, 0],
    cp2: [0.58, 1],
    snap: '1',
    controlPreset: 'Ambient',
  },
];
