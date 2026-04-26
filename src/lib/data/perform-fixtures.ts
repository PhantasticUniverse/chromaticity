/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Perform-view fixtures — TypeScript mirror of the v0.2 bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
 * `PerformView` component (variant === "classic"). Powers the W18 PR1
 * full port that replaces the W12 PR3 single-page IPC dev surface
 * (control 0 slider, snap grid, preset / playlist controls, widgets
 * showcase) with the canonical SceneControls + MetaControls + MediaStrip
 * + Preview + ScenePanel layout.
 *
 * Per the auto-memory `feedback_wireframes_normative_for_ui_ux.md`, the
 * bundle JSX is the canonical structural reference; numeric values and
 * widget order below mirror the JSX verbatim so future bundle revisions
 * sync cleanly through both surfaces.
 *
 * W18 PR1 posture: hardcoded fixtures only. Real-time control wiring
 * (Fader / Toggle / Knob / Swatch dispatch into the runtime) is
 * Phase-3+/4a `EV-INTEGRATION-MATRIX` scope; preset / scene-edit IPC
 * is Phase-3 `EV-PRESET-PLAYLIST` scope. Setlist rows reuse
 * `SETLIST_FIXTURE` from `setlist-fixtures.ts`, the same fixture the
 * Library view's right-sidebar setlist panel consumes.
 *
 * `tint` strings reference the design-token contract in
 * `src/lib/tokens/tokens.css`. The bundle uses `var(--cat-*)` short
 * names; this fixture references the production-side `var(--color-cat-*)`
 * names, which the three theme cascades (Pulse / Atelier / Console)
 * each redefine.
 */

/** A single widget row inside a SceneControls / MetaControls ControlGroup. */
export type PerformControlSpec =
  | {
      readonly kind: 'toggle';
      readonly label: string;
      readonly on: boolean;
    }
  | {
      readonly kind: 'fader';
      readonly label: string;
      readonly value: number;
      /** Bundle's `tickAt` mark — drawn as a small mid-track tick. */
      readonly tickAt?: number;
    }
  | {
      readonly kind: 'knob';
      readonly label: string;
      readonly value: number;
    }
  | {
      readonly kind: 'swatch';
      readonly label: string;
      readonly color: string;
    };

/** A control-group column inside a panel body row-scroll layout. */
export interface PerformControlGroup {
  readonly name: string;
  /** CSS colour string for the group's top-border tint and header label. */
  readonly tint: string;
  readonly controls: readonly PerformControlSpec[];
}

/**
 * SceneControls panel rows — bundle JSX lines 18-38 (`PerformView`).
 * Header reads "Scene Controls · Glitch" with Lock-all / Default / Random
 * action buttons; body row-scrolls 5 ControlGroups.
 */
export const SCENE_CONTROL_GROUPS: readonly PerformControlGroup[] = [
  {
    name: 'action',
    tint: 'var(--color-cat-color)',
    controls: [
      { kind: 'toggle', label: 'pusher', on: true },
      { kind: 'toggle', label: 'scatter', on: false },
    ],
  },
  {
    name: 'effects',
    tint: 'var(--color-cat-motion)',
    controls: [
      { kind: 'toggle', label: 'soften', on: false },
      { kind: 'toggle', label: 'monochrome', on: true },
    ],
  },
  {
    name: 'speed',
    tint: 'var(--color-cat-fx)',
    controls: [
      { kind: 'toggle', label: 'vert speed', on: true },
      { kind: 'toggle', label: 'hor speed', on: true },
    ],
  },
  {
    name: 'push',
    tint: 'var(--color-cat-media)',
    controls: [
      { kind: 'toggle', label: 'auto push', on: true },
      { kind: 'fader', label: 'manual push', value: 0.62 },
    ],
  },
  {
    name: 'dive',
    tint: 'var(--color-cat-limit)',
    controls: [{ kind: 'fader', label: 'dive rate', value: 0.0 }],
  },
];

/**
 * MetaControls panel rows — bundle JSX lines 50-82.
 * Header reads "Meta Controls" with Default / Random action buttons; body
 * row-scrolls 6 ControlGroups (color, mirror, media, video, limit colors,
 * general).
 */
export const META_CONTROL_GROUPS: readonly PerformControlGroup[] = [
  {
    name: 'color',
    tint: 'var(--color-cat-color)',
    controls: [
      { kind: 'toggle', label: 'invert', on: false },
      { kind: 'fader', label: 'brightness', value: 0.0, tickAt: 0.5 },
      { kind: 'fader', label: 'gamma', value: 1.0, tickAt: 0.5 },
      { kind: 'fader', label: 'contrast', value: 1.05, tickAt: 0.5 },
      { kind: 'fader', label: 'hue', value: 0.38, tickAt: 0.5 },
      { kind: 'fader', label: 'saturation', value: 0.75, tickAt: 0.5 },
    ],
  },
  {
    name: 'mirror',
    tint: 'var(--color-cat-motion)',
    controls: [
      { kind: 'toggle', label: 'vert mirror', on: false },
      { kind: 'toggle', label: 'hor mirror', on: false },
    ],
  },
  {
    name: 'media',
    tint: 'var(--color-cat-fx)',
    controls: [
      { kind: 'toggle', label: 'invert media', on: true },
      { kind: 'toggle', label: 'vert flip', on: false },
      { kind: 'toggle', label: 'hor flip', on: true },
      { kind: 'fader', label: 'media contrast', value: 0.18 },
    ],
  },
  {
    name: 'video',
    tint: 'var(--color-cat-media)',
    controls: [
      { kind: 'toggle', label: 'paused', on: false },
      { kind: 'fader', label: 'playback speed', value: 0.5, tickAt: 0.5 },
    ],
  },
  {
    name: 'limit colors',
    tint: 'var(--color-cat-limit)',
    controls: [
      { kind: 'toggle', label: 'limit colors', on: false },
      { kind: 'swatch', label: 'low color', color: '#ff4d5e' },
      { kind: 'swatch', label: 'high color', color: '#ffb64d' },
    ],
  },
  {
    name: 'general',
    tint: 'var(--color-cat-geom)',
    controls: [
      { kind: 'toggle', label: 'alpha channel', on: false },
      { kind: 'knob', label: 'transition', value: 0.25 },
    ],
  },
];

/** Media strip card — `kind: 'none'` is a disabled tile, `'webcam'` is a
 * live-source tile (webcam / display capture), `'video'` is a procedurally
 * generated thumbnail surrogate, `'test'` is the W21 PR2 static SMPTE
 * test-card calibration pattern (no per-frame animation; no rendering
 * backend dependency). When the test-card tile is active, the Perform
 * view's Preview hero renders the `<TestCard>` widget instead of the
 * gradient-blob placeholder. */
export type MediaCardKind = 'none' | 'webcam' | 'video' | 'test';

export interface PerformMediaCard {
  readonly id: string;
  readonly label: string;
  readonly kind: MediaCardKind;
  /** Per-tile variation seed — drives the placeholder thumbnail tinting.
   * Mirrors the bundle's `seed={i * 7 + 3}` formula for video tiles; for
   * `'none'` and `'webcam'` tiles the seed is ignored at render time. */
  readonly seed: number;
}

/** Bundle JSX lines 96-103 — first three slots are none / webcam / display,
 * the W21 PR2 fourth slot is the static Test Card calibration pattern,
 * remaining 12 slots are procedurally-named videos with `seed = i*7+3`.
 * `activeMedia === 1` per `React.useState(1)` initial state, so the second
 * card (webcam) renders as selected by default. */
export const MEDIA_CARDS: readonly PerformMediaCard[] = [
  { id: 'm-none', label: 'none', kind: 'none', seed: 0 },
  { id: 'm-webcam', label: 'webcam', kind: 'webcam', seed: 0 },
  { id: 'm-display', label: 'display', kind: 'webcam', seed: 0 },
  { id: 'm-test', label: 'test card', kind: 'test', seed: 0 },
  ...[
    'jellyfish.mp4',
    'RoadTrip.mp4',
    'sand.mov',
    'star_trails.mov',
    'glossy.mp4',
    'growing_shapes.mp4',
    'mirror_loop.mp4',
    'noise.mp4',
    'CloudAtlas.mp4',
    'hadopelagic.mov',
    'waterflow.mp4',
    'texture_01.mp4',
  ].map((label, i) => ({
    id: `m-video-${i}`,
    label,
    kind: 'video' as const,
    seed: i * 7 + 3,
  })),
];

/** Default selected media-card index — matches the bundle's
 * `React.useState(1)` initial state (the second tile, "webcam"). */
export const MEDIA_DEFAULT_ACTIVE_INDEX = 1;

/** Media strip tab pills — bundle JSX line 90-92. */
export const MEDIA_TABS = ['all', 'live', 'media'] as const;
export type MediaTab = (typeof MEDIA_TABS)[number];

/** Setlist dropdown options — bundle JSX lines 124-128. */
export const SETLIST_OPTIONS: readonly string[] = [
  'Demo Scenes',
  'Festival Set — Dusk',
  'Ambient Pool',
];
