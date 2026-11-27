/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * OSC route taxonomy fixtures — TypeScript mirror of the predev bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/osc.jsx`
 * `OSC_ROUTES` constant, byte-aligned with `docs/08-io-integrations.md`
 * §OSC normative spec.
 *
 * Upstream normative source: `docs/08-io-integrations.md` §OSC. The
 * bundle is the design reference; this module is the production-side
 * source for the W14 OSC view's routes table.
 *
 * W14 PR4 posture: HARDCODED fixtures only. Live route registration
 * (dynamic enumeration of which routes are bound at runtime) is
 * Phase-3 `EV-INTEGRATION-MATRIX` scope. The wireframe-06 layout is
 * the immediate goal; backend wiring follows.
 *
 * Per the `feedback_wireframes_normative_for_ui_ux.md` memory, this
 * file's structure mirrors the bundle's JSX so future updates to the
 * spec sync cleanly through both surfaces.
 */

export type OscDirection = 'in' | 'out' | 'both';
export type OscFamily = 'control' | 'scene' | 'preset' | 'snap' | 'playlist' | 'audio' | 'reserved';

export interface OscRouteFixture {
  /** Plural-root v1 canonical address. */
  readonly canonical: string;
  /** Compat-syn-v1 single-root alias when the compat toggle is on; null otherwise. */
  readonly compatAlias: string | null;
  /** OSC type tag summary per docs/08 — `f`/`i`/`s`/`b`/`[s]`/`s,i` or `—` for triggers. */
  readonly type: string;
  /** Human-readable value range / description. */
  readonly range: string;
  /** Direction relative to the chromaticity adapter. */
  readonly direction: OscDirection;
  /** Route family for the family-tint pill. */
  readonly family: OscFamily;
  /** Optional clarifying note rendered below the row in expanded views. */
  readonly note?: string;
}

/**
 * Default `/audio/var/*` family enable state per docs/08 §Audio-variable
 * output routes. LVL/HIT/BEAT default-on; TIM/BPM/LRG default-off.
 */
export interface OscAudioVarFamily {
  readonly id: string;
  readonly label: string;
  readonly defaultOn: boolean;
  readonly examples: readonly string[];
}

export const OSC_AUDIO_VAR_FAMILIES: readonly OscAudioVarFamily[] = [
  {
    id: 'LVL',
    label: 'Levels',
    defaultOn: true,
    examples: ['bassLevel', 'midLevel', 'highLevel'],
  },
  {
    id: 'HIT',
    label: 'Hits',
    defaultOn: true,
    examples: ['bassHits', 'midHits', 'highHits'],
  },
  {
    id: 'BEAT',
    label: 'Beat',
    defaultOn: true,
    examples: ['onBeat', 'toggleOnBeat', 'randomOnBeat'],
  },
  {
    id: 'TIM',
    label: 'Time',
    defaultOn: false,
    examples: ['time', 'curvedTime'],
  },
  {
    id: 'BPM',
    label: 'BPM',
    defaultOn: false,
    examples: ['bpm', 'bpmSin', 'bpmTri'],
  },
  {
    id: 'LRG',
    label: 'Long-range',
    defaultOn: false,
    examples: ['fadeInOut', 'intensity'],
  },
];

/** Reserved prefixes per OSC-CONF-16 — inbound rejected with `prefix-reserved`. */
export const OSC_RESERVED_PREFIXES: readonly string[] = ['/data/*', '/data/subscribe/*'];

export const OSC_ROUTES: readonly OscRouteFixture[] = [
  {
    canonical: '/controls/{bank}/{name}',
    compatAlias: '/controls/{addr}',
    type: 'f',
    range: '0.0 – 1.0',
    family: 'control',
    direction: 'both',
  },
  {
    canonical: '/controls/{bank}/{name}/numoptions',
    compatAlias: null,
    type: 'i',
    range: 'N (enum count)',
    family: 'control',
    direction: 'out',
    note: 'dropdown discovery',
  },
  {
    canonical: '/controls/{bank}/{name}/label/{index}',
    compatAlias: '/controls/{addr}/label {s}',
    type: 's',
    range: 'label text',
    family: 'control',
    direction: 'out',
    note: 'serial dual-emit under compat-syn-v1',
  },
  {
    canonical: '/scenes',
    compatAlias: null,
    type: '[s]',
    range: 'list, outbound',
    family: 'scene',
    direction: 'out',
  },
  {
    canonical: '/scenes/{sceneName}',
    compatAlias: '/scene/{sceneName}',
    type: '—',
    range: 'announce / trigger',
    family: 'scene',
    direction: 'both',
  },
  {
    canonical: '/presets',
    compatAlias: null,
    type: '[s]',
    range: 'list, outbound',
    family: 'preset',
    direction: 'out',
  },
  {
    canonical: '/presets/{name}',
    compatAlias: '/preset/{name}',
    type: '—',
    range: 'apply',
    family: 'preset',
    direction: 'in',
  },
  {
    canonical: '/presets/new {name}',
    compatAlias: null,
    type: 's',
    range: 'save-as',
    family: 'preset',
    direction: 'in',
  },
  {
    canonical: '/favslots/{n}',
    compatAlias: '/favslot/{n}',
    type: '—',
    range: 'Snap recall (n 0–15)',
    family: 'snap',
    direction: 'in',
    note: 'wire literal `favslot` retained only as compat alias; UI = Snap',
  },
  {
    canonical: '/favslots/{n}/save',
    compatAlias: '/favslots/{n} save',
    type: '—',
    range: 'Snap save',
    family: 'snap',
    direction: 'in',
  },
  {
    canonical: '/favslots/{n}/clear',
    compatAlias: '/favslots/{n} clear',
    type: '—',
    range: 'Snap clear',
    family: 'snap',
    direction: 'in',
  },
  {
    canonical: '/favslots/{n}/name',
    compatAlias: null,
    type: 's',
    range: 'name readout',
    family: 'snap',
    direction: 'out',
  },
  {
    canonical: '/playlists',
    compatAlias: null,
    type: '[s]',
    range: 'list, outbound',
    family: 'playlist',
    direction: 'out',
  },
  {
    canonical: '/playlists/next',
    compatAlias: '/playlist/next',
    type: '—',
    range: 'trigger',
    family: 'playlist',
    direction: 'in',
  },
  {
    canonical: '/playlists/position {i}',
    compatAlias: null,
    type: 'i',
    range: 'neg counts from end',
    family: 'playlist',
    direction: 'both',
  },
  {
    canonical: '/playlists/toggleplay',
    compatAlias: '/playlist/toggleplay',
    type: '—',
    range: 'trigger',
    family: 'playlist',
    direction: 'in',
  },
  {
    canonical: '/audio/var/{uniform}',
    compatAlias: null,
    type: 'f',
    range: 'per-uniform cadence',
    family: 'audio',
    direction: 'out',
    note: 'syn_BassLevel → /audio/var/bassLevel',
  },
  {
    canonical: '/audio/threshold {f}',
    compatAlias: null,
    type: 'f',
    range: '0.0 – 1.0',
    family: 'audio',
    direction: 'both',
    note: 'inbound sets gate; outbound announces current threshold value',
  },
  {
    canonical: '/data/*',
    compatAlias: null,
    type: '—',
    range: 'reserved post-MVP',
    family: 'reserved',
    direction: 'in',
    note: 'rejects with prefix-reserved diagnostic',
  },
  {
    canonical: '/data/subscribe/*',
    compatAlias: null,
    type: '—',
    range: 'reserved post-MVP',
    family: 'reserved',
    direction: 'in',
    note: 'rejects with prefix-reserved diagnostic',
  },
];

/**
 * Sample live-log fixture per the wireframe-06 LIVE LOG section.
 * Real streaming via OSC-adapter event subscription is Phase-3 scope.
 */
export interface OscLiveLogEntry {
  readonly time: string;
  readonly direction: 'in' | 'out';
  readonly route: string;
  readonly value: string;
}

export const OSC_LIVE_LOG_FIXTURE: readonly OscLiveLogEntry[] = [
  { time: '12:04:20', direction: 'in', route: '/controls/scene/pusher', value: '0.62' },
  { time: '12:04:21', direction: 'in', route: '/controls/meta/hue', value: '0.30' },
  { time: '12:04:22', direction: 'in', route: '/audio/threshold', value: '0.32' },
  { time: '12:04:23', direction: 'in', route: '/audio/var/bassLevel', value: '0.44' },
  { time: '12:04:24', direction: 'in', route: '/favslots/1', value: 'recall' },
  { time: '12:04:25', direction: 'out', route: '/scenes/Glitch', value: 'announce' },
  { time: '12:04:26', direction: 'in', route: '/controls/meta/brightness', value: '0.55' },
  { time: '12:04:27', direction: 'out', route: '/audio/var/midHits', value: '0.71' },
];
