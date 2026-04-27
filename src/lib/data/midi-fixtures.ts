/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * MIDI view fixtures — TypeScript mirror of the predev bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/midi.jsx`
 * `MidiView` component. Mirrors the M-01..M-14 surface declarations
 * called out in the bundle's module-level prose.
 *
 * W18 PR3 posture: HARDCODED frozen-frame fixtures only. The bundle's
 * `requestAnimationFrame` driven `MidiLearnWaveform`, the polling
 * `MidiHistoryStrip`, and the 250ms learn-idle timer all become
 * static snapshots. Real-time refresh waits for the MIDI integration
 * adapter wiring (post-Phase-3 `EV-INTEGRATION-MATRIX` scope per
 * docs/26).
 *
 * Per the wireframe-alignment memory
 * (`feedback_wireframes_normative_for_ui_ux.md`), the bundle JSX is
 * the canonical structural reference. Numeric values, hex codes,
 * device names, and copy strings below are sampled verbatim from
 * the bundle's seed data and arrays so the port stays visually
 * aligned to wireframe-05 (`05-mappings-midi-pads.png`).
 *
 * Design-token mapping for hex codes that appear in the bundle:
 *   - Gold preset slot 0 ribbon: `--color-warning` (amber/gold token).
 *   - Bundle's `hsl(${hue} 90% 55%)` virtual-pad colours retained as
 *     literal HSL since no token cascade encodes the rainbow gradient.
 */

/* ---------- Devices (M-01 / M-13) ---------- */

export type MidiDeviceDirection = 'in' | 'out' | 'in+out';

/**
 * Optional pad-grid metadata for pad-style controllers.
 *
 * The MIDI Virtual Pad mirror in the M-14 panel renders a `CuePad` grid
 * sized to the selected controller's hardware. Controllers without
 * `padDims` are not pad-capable (keyboards, USB cables, OSC bridges) and
 * are filtered out of the pad-controller selector.
 */
export interface MidiDevicePadDims {
  readonly rows: number;
  readonly cols: number;
}

export interface MidiDevice {
  readonly id: string;
  readonly name: string;
  readonly vendor: string;
  readonly direction: MidiDeviceDirection;
  readonly allowed: boolean;
  readonly maps: number;
  readonly mpeWarned: boolean;
  readonly padDims?: MidiDevicePadDims;
}

export const MIDI_DEVICES: readonly MidiDevice[] = [
  {
    id: 'dev.lpx',
    name: 'Launchpad X',
    vendor: 'Novation',
    direction: 'in+out',
    allowed: true,
    maps: 42,
    mpeWarned: false,
    padDims: { rows: 8, cols: 8 },
  },
  {
    id: 'dev.apc',
    name: 'APC40 mkII',
    vendor: 'Akai',
    direction: 'in',
    allowed: true,
    maps: 18,
    mpeWarned: false,
    padDims: { rows: 5, cols: 8 },
  },
  {
    id: 'dev.push2',
    name: 'Push 2',
    vendor: 'Ableton',
    direction: 'in+out',
    allowed: true,
    maps: 0,
    mpeWarned: false,
    padDims: { rows: 8, cols: 8 },
  },
  {
    id: 'dev.lpp3',
    name: 'Launchpad Pro Mk3',
    vendor: 'Novation',
    direction: 'in+out',
    allowed: true,
    maps: 0,
    mpeWarned: false,
    padDims: { rows: 8, cols: 8 },
  },
  {
    id: 'dev.apcmini',
    name: 'APC mini Mk2',
    vendor: 'Akai',
    direction: 'in',
    allowed: false,
    maps: 0,
    mpeWarned: false,
    padDims: { rows: 8, cols: 8 },
  },
  {
    id: 'dev.touch',
    name: 'iPad · TouchOSC Bridge',
    vendor: 'hexler',
    direction: 'in',
    allowed: true,
    maps: 4,
    mpeWarned: false,
  },
  {
    id: 'dev.seab',
    name: 'Seaboard RISE 49',
    vendor: 'ROLI',
    direction: 'in',
    allowed: false,
    maps: 0,
    mpeWarned: true,
  },
  {
    id: 'dev.mio',
    name: 'USB MIDI Cable',
    vendor: '—',
    direction: 'in',
    allowed: false,
    maps: 0,
    mpeWarned: false,
  },
] as const;

/* ---------- Tabs ---------- */

export type MidiTabId = 'scene' | 'global' | 'meta' | 'media' | 'snaps' | 'start' | 'devices';

export interface MidiTab {
  readonly id: MidiTabId;
  readonly label: string;
}

export const MIDI_TABS: readonly MidiTab[] = [
  { id: 'scene', label: 'Scene-specific' },
  { id: 'global', label: 'Global' },
  { id: 'meta', label: 'Meta controls' },
  { id: 'media', label: 'Media' },
  { id: 'snaps', label: 'Snaps' },
  { id: 'start', label: 'Start-scenes' },
  { id: 'devices', label: 'Devices' },
] as const;

/* ---------- Binding rows (Scene / Global / Meta / Media / Start tabs) ---------- */

export type MidiBindingMode = 'absolute' | 'relative' | 'toggle' | 'momentary';
export type MidiBindingFlag = '14-bit' | "2's-comp" | null;
export type MidiBindingFamily = 'scene' | 'meta' | 'media' | 'start';

export interface MidiBindingRow {
  readonly target: string;
  readonly device: string;
  readonly channel: string;
  readonly msg: 'CC' | 'Note';
  readonly num: string;
  readonly mode: MidiBindingMode;
  readonly min: string;
  readonly max: string;
  readonly threshold: string;
  readonly family: MidiBindingFamily;
  readonly flag: MidiBindingFlag;
}

export const MIDI_ROWS_BY_TAB: Readonly<Record<MidiTabId, readonly MidiBindingRow[]>> = {
  scene: [
    {
      target: 'scene.pusher',
      device: 'Launchpad X',
      channel: '1',
      msg: 'CC',
      num: '21',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.05',
      family: 'scene',
      flag: null,
    },
    {
      target: 'scene.palette',
      device: 'Launchpad X',
      channel: '1',
      msg: 'CC',
      num: '22',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.05',
      family: 'scene',
      flag: null,
    },
    {
      target: 'scene.dive',
      device: 'Launchpad X',
      channel: '2',
      msg: 'Note',
      num: '42',
      mode: 'momentary',
      min: '0.00',
      max: '1.00',
      threshold: '0.10',
      family: 'scene',
      flag: null,
    },
  ],
  global: [
    {
      target: 'meta.brightness',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '14',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: null,
    },
    {
      target: 'meta.hue',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '15',
      mode: 'relative',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: "2's-comp",
    },
    {
      target: 'meta.contrast',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '16/48',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: '14-bit',
    },
  ],
  meta: [
    {
      target: 'meta.brightness',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '14',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: null,
    },
    {
      target: 'meta.hue',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '15',
      mode: 'relative',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: "2's-comp",
    },
    {
      target: 'meta.saturation',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '17',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: null,
    },
    {
      target: 'meta.contrast',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '16/48',
      mode: 'absolute',
      min: '0.00',
      max: '1.00',
      threshold: '0.02',
      family: 'meta',
      flag: '14-bit',
    },
  ],
  media: [
    {
      target: 'media.speed',
      device: 'iPad · TouchOSC',
      channel: '1',
      msg: 'CC',
      num: '1',
      mode: 'absolute',
      min: '0.00',
      max: '2.00',
      threshold: '0.00',
      family: 'media',
      flag: null,
    },
    {
      target: 'media.invert',
      device: 'APC40 mkII',
      channel: '1',
      msg: 'CC',
      num: '18',
      mode: 'toggle',
      min: '0.00',
      max: '1.00',
      threshold: '—',
      family: 'media',
      flag: null,
    },
  ],
  start: [
    {
      target: 'start.Intro',
      device: 'Launchpad X',
      channel: '1',
      msg: 'Note',
      num: 'C1',
      mode: 'momentary',
      min: '—',
      max: '—',
      threshold: '—',
      family: 'start',
      flag: null,
    },
    {
      target: 'start.Build',
      device: 'Launchpad X',
      channel: '1',
      msg: 'Note',
      num: 'D1',
      mode: 'momentary',
      min: '—',
      max: '—',
      threshold: '—',
      family: 'start',
      flag: null,
    },
    {
      target: 'start.Drop',
      device: 'Launchpad X',
      channel: '1',
      msg: 'Note',
      num: 'E1',
      mode: 'momentary',
      min: '—',
      max: '—',
      threshold: '—',
      family: 'start',
      flag: null,
    },
  ],
  // Snaps + devices are rendered by dedicated panels — no binding rows.
  snaps: [],
  devices: [],
};

export const MIDI_TAB_HINTS: Readonly<Record<MidiTabId, string>> = {
  scene: 'Scene-specific mappings override Global for the current scene.',
  global: 'Global mappings apply to all scenes unless overridden.',
  meta: 'Meta mappings target brightness / hue / saturation / contrast pre-output.',
  media: 'Media mappings target playback speed / invert / fit-or-zoom.',
  snaps: '16 Snap slots — slot 0 is the gold preset (CP-05). Wire literal: favslot.',
  start: 'Start-scenes bind a scene# → note, optionally with a preset.',
  devices: 'Explicit allowlist — Chromaticity never auto-claims a device (M-01).',
};

/* ---------- Snap slots (M-09) ---------- */

export interface MidiSnapSlot {
  readonly n: number;
  readonly name: string;
  readonly binding: string | null;
  readonly mode: 'toggle' | 'momentary';
  readonly isGold: boolean;
  readonly isActive: boolean;
}

export const MIDI_SNAP_SLOTS: readonly MidiSnapSlot[] = Array.from({ length: 16 }, (_, i) => {
  const namesByIdx: Readonly<Record<number, string>> = {
    0: 'Start (gold preset)',
    1: 'Intro wash',
    2: 'Build',
    3: 'Drop',
    7: 'Warehouse · Peak',
  };
  const bindingsByIdx: Readonly<Record<number, string>> = {
    1: 'LPX · Note 81',
    2: 'LPX · Note 82',
    3: 'LPX · Note 83',
    7: 'APC · Note 36',
  };
  return {
    n: i,
    name: namesByIdx[i] ?? '',
    binding: bindingsByIdx[i] ?? null,
    mode: i === 3 ? 'momentary' : 'toggle',
    isGold: i === 0,
    isActive: i === 2,
  } as const;
});

/* ---------- BPM Sync source (M-10) ---------- */

export type BpmSyncSourceId = 'link' | 'midiclock' | 'audio' | 'tap';
export type BpmSyncState = 'active' | 'standby' | 'idle';

export interface BpmSyncSource {
  readonly id: BpmSyncSourceId;
  readonly name: string;
  readonly detail: string;
  readonly state: BpmSyncState;
}

export const BPM_SYNC_SOURCES: readonly BpmSyncSource[] = [
  { id: 'link', name: 'Ableton Link', detail: '4 peers · 128.00 BPM', state: 'active' },
  { id: 'midiclock', name: 'MIDI Clock', detail: 'APC40 · idle', state: 'standby' },
  { id: 'audio', name: 'Audio estimate', detail: '128.3 BPM · conf 0.82', state: 'standby' },
  { id: 'tap', name: 'Tap tempo', detail: 'last: —', state: 'idle' },
] as const;

/* ---------- MTC readout (M-08) ---------- */

export interface MtcReadout {
  readonly timecode: string;
  readonly framerate: string;
  readonly assemblyState: string;
  readonly uniformName: string;
}

export const MIDI_MTC: MtcReadout = {
  timecode: '01:04:22:18',
  framerate: '29.97 fps',
  assemblyState: 'locked',
  // Per `docs/36-reactive-symbol-catalog.md` namespace governance: new
  // feature work uses `chroma_*` as canonical. The MTC uniform is a new
  // symbol the W18 PR3 sub-agent introduced; it was not in the catalog
  // under any prefix, so the catalog-aligned native name is `chroma_Timecode`.
  // A `compat-syn-v1` alias would only land if MTC continuity with the
  // reference is required; that's deferred until the audio/MIDI adapters
  // wire to runtime IPC (Phase-3+).
  uniformName: 'chroma_Timecode',
};

/* ---------- History strip (M-03) ---------- */

export interface MidiHistoryEntry {
  readonly id: string;
  readonly msg: string;
  readonly device: string;
  readonly hex: string;
  readonly value: number;
  readonly relativeMs: number;
}

export const MIDI_HISTORY_FIXTURE: readonly MidiHistoryEntry[] = [
  { id: 'h0', msg: 'CC 15 MSB', device: 'APC40', hex: '0x42', value: 66, relativeMs: 180 },
  { id: 'h1', msg: 'CC 47 LSB', device: 'APC40', hex: '0x31', value: 49, relativeMs: 200 },
  { id: 'h2', msg: 'CC 15 MSB', device: 'APC40', hex: '0x3f', value: 63, relativeMs: 420 },
  { id: 'h3', msg: 'CC 15 MSB', device: 'APC40', hex: '0x39', value: 57, relativeMs: 640 },
  { id: 'h4', msg: 'Note 42', device: 'LPX', hex: '0x2a', value: 112, relativeMs: 1380 },
] as const;

/* ---------- Learn-mode panel snapshot (M-03 / M-04) ---------- */

export type RelativeEncoderMode =
  | 'auto'
  | 'absolute'
  | '7F-relative'
  | "2's-complement"
  | 'binary-offset';

export interface MidiLearnSnapshot {
  readonly target: string;
  readonly lastInputDevice: string;
  readonly lastInputChannel: string;
  readonly lastInputMessage: string;
  readonly valueHex: string;
  readonly valueDec: number;
  readonly waveform: readonly number[];
  readonly detectedMode: RelativeEncoderMode;
}

/*
 * Frozen waveform sampled from the bundle's `MidiLearnWaveform` signal
 * generator at `t ≈ 17 s` — matches the 160-point ring buffer the bundle
 * fills per-frame. The values are the same `0.45 + sin(t·0.9)·0.22 +
 * sin(t·2.3)·0.08 + spike` formula evaluated at deterministic timestamps,
 * so the SVG path renders identically across mounts.
 */
const LEARN_WAVEFORM_SAMPLES: readonly number[] = (() => {
  // 80 samples is enough for a 100-unit-wide SVG to render a detailed
  // waveform without bloating the fixture file. The bundle uses 160 in a
  // canvas; the inline SVG densifies less aggressively.
  const N = 80;
  const out: number[] = [];
  for (let i = 0; i < N; i++) {
    const t = 14 + (i / N) * 3; // sweep t = 14..17 s
    const base = 0.45 + Math.sin(t * 0.9) * 0.22 + Math.sin(t * 2.3) * 0.08;
    // Deterministic pseudo-spike on every 11th sample so the snapshot is
    // not dead-flat but is reproducible across mounts.
    const spike = i % 11 === 7 ? 0.18 : 0;
    out.push(Math.max(0, Math.min(1, base + spike)));
  }
  return out;
})();

export const MIDI_LEARN_FIXTURE: MidiLearnSnapshot = {
  target: 'meta.hue',
  lastInputDevice: 'APC40',
  lastInputChannel: '1',
  lastInputMessage: 'CC 15 MSB',
  valueHex: '0x42',
  valueDec: 66,
  waveform: LEARN_WAVEFORM_SAMPLES,
  detectedMode: "2's-complement",
};

export const RELATIVE_ENCODER_MODES: readonly RelativeEncoderMode[] = [
  'auto',
  'absolute',
  '7F-relative',
  "2's-complement",
  'binary-offset',
] as const;

/* ---------- Virtual pad (M-14) ---------- */

export interface VirtualPadCell {
  readonly index: number;
  readonly hue: number;
  readonly on: boolean;
}

const PAD_ON_INDICES: readonly number[] = [
  0, 1, 4, 7, 10, 13, 19, 22, 26, 29, 34, 37, 43, 46, 52, 58,
];

export const VIRTUAL_PAD_CELLS: readonly VirtualPadCell[] = Array.from({ length: 64 }, (_, i) => ({
  index: i,
  hue: 30 + (i % 8) * 12,
  on: PAD_ON_INDICES.includes(i),
}));

/* ---------- MPE banner (M-07) ---------- */

export const MPE_BANNER_COPY =
  'MPE-configuration message observed from Seaboard RISE 49 — no global state corrupted (MIDI-CONF-18). Diagnostic fires once per session.';

/* ---------- Operator capability hints (used as `data-cap` attributes) ---------- */

/*
 * Mirror of the bundle's `cap="io.midi.input"` / `cap="io.midi.output"`
 * pattern. Until the entitlement infrastructure lands (W14+), the view
 * only annotates rows with `data-cap` — it never hides them.
 */
export const MIDI_CAPABILITY = {
  input: 'io.midi.input',
  output: 'io.midi.output',
  learn: 'io.midi.learn',
} as const;
