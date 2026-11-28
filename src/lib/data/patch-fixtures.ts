/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * Patch view fixtures — TypeScript mirror of the predev bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/patch.jsx`
 * `PatchView` component. The bundle is the canonical structural source
 * per the wireframe-alignment memory (`feedback_wireframes_normative_for_ui_ux.md`).
 *
 * W19 PR1 posture: HARDCODED frozen-frame fixtures only. The bundle's
 * React-managed selection / drag / commit interactivity is out of scope;
 * the W19 port renders the same node graph as a static visual stub.
 * Real-time refresh and edit affordances wait for the patch-graph
 * runtime adapter (post-Phase-3 scope).
 *
 * Design-token mapping for the bundle's CSS custom properties:
 *   --accent-info     → --color-info
 *   --accent-primary  → --color-primary
 *   --magenta         → --color-cat-color
 *   --violet          → --color-cat-limit
 *   --orange          → --color-warning
 *
 * The bundle's geometry constants (HEADER_H = 29, ROW_H = 22, port
 * circle offsets at -5/+5 of the node edge) are mirrored verbatim so
 * pin-position math stays identical between port and bundle.
 */

/* ---------- Node graph geometry (kept in sync with the DOM) ---------- */

/** Node header height in pixels (padding 6px 10px + caps text + 1px border). */
export const PATCH_HEADER_H = 29;

/** Each pin row's height in pixels. */
export const PATCH_ROW_H = 22;

/* ---------- Node kinds and category tints ---------- */

export type PatchNodeKind = 'source' | 'scene' | 'fx' | 'mod' | 'output';

/**
 * Per-kind tint mapping. Mirrors the bundle's `kindColor` object,
 * remapped to Chromaticity tokens. Used both as the node header
 * accent stripe and as the port-circle border colour.
 */
export const PATCH_KIND_TINT: Readonly<Record<PatchNodeKind, string>> = {
  source: 'var(--color-info)',
  scene: 'var(--color-primary)',
  fx: 'var(--color-cat-color)',
  mod: 'var(--color-cat-limit)',
  output: 'var(--color-warning)',
};

/* ---------- Nodes ---------- */

export interface PatchNode {
  readonly id: string;
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly title: string;
  readonly kind: PatchNodeKind;
  readonly pins: readonly string[];
}

/**
 * Mutable mirror of {@link PatchNode}. The W31 port keeps the
 * canonical {@link PATCH_NODES} as the read-only seed; the
 * component clones into a `MutablePatchNode[]` `$state` for live
 * pointer-drag repositioning. `pins` stays `readonly` because pin
 * topology is not mutated by drag (only `x` / `y`).
 */
export interface MutablePatchNode {
  readonly id: string;
  x: number;
  y: number;
  readonly w: number;
  readonly title: string;
  readonly kind: PatchNodeKind;
  readonly pins: readonly string[];
}

export const PATCH_NODES: readonly PatchNode[] = [
  {
    id: 'audio',
    x: 40,
    y: 80,
    w: 200,
    title: 'Audio Input',
    kind: 'source',
    pins: ['bass', 'mid', 'midhi', 'high', 'rms'],
  },
  {
    id: 'scene',
    x: 290,
    y: 40,
    w: 220,
    title: 'Scene: Glitch',
    kind: 'scene',
    pins: ['pusher', 'dive', 'auto push', 'manual push', 'speed h', 'speed v'],
  },
  {
    id: 'color',
    x: 560,
    y: 200,
    w: 220,
    title: 'Color Processor',
    kind: 'fx',
    pins: ['brightness', 'hue', 'saturation', 'contrast'],
  },
  {
    id: 'lfo',
    x: 40,
    y: 300,
    w: 200,
    title: 'LFO · 1/4',
    kind: 'mod',
    pins: ['out'],
  },
  {
    id: 'midi',
    x: 40,
    y: 420,
    w: 200,
    title: 'MIDI · Launchpad X',
    kind: 'source',
    pins: ['cc 21', 'cc 14', 'note 42'],
  },
  {
    id: 'out',
    x: 820,
    y: 120,
    w: 200,
    title: 'Stage Out',
    kind: 'output',
    pins: ['display 2', 'syphon', 'record'],
  },
];

/* ---------- Wires ---------- */

export interface PatchWire {
  readonly from: readonly [nodeId: string, pinName: string];
  readonly to: readonly [nodeId: string, pinName: string];
  readonly color: string;
}

export const PATCH_WIRES: readonly PatchWire[] = [
  { from: ['audio', 'bass'], to: ['scene', 'pusher'], color: 'var(--color-warning)' },
  { from: ['audio', 'mid'], to: ['color', 'hue'], color: 'var(--color-cat-color)' },
  { from: ['lfo', 'out'], to: ['scene', 'dive'], color: 'var(--color-info)' },
  { from: ['midi', 'cc 14'], to: ['color', 'brightness'], color: 'var(--color-primary)' },
  {
    from: ['scene', 'speed h'],
    to: ['color', 'saturation'],
    color: 'var(--color-cat-limit)',
  },
  {
    from: ['color', 'contrast'],
    to: ['out', 'display 2'],
    color: 'var(--color-primary)',
  },
];

/* ---------- Palette (left column) ---------- */

export interface PatchPaletteCategory {
  readonly category: string;
  readonly items: readonly string[];
}

export const PATCH_PALETTE: readonly PatchPaletteCategory[] = [
  { category: 'Sources', items: ['Audio Input', 'MIDI', 'OSC', 'Ableton Link', 'DMX'] },
  { category: 'Modulators', items: ['LFO', 'Envelope', 'Random', 'Sequencer', 'Math'] },
  { category: 'Scenes', items: ['Current', 'Glitch', 'Quicksand', 'Glob'] },
  { category: 'Processors', items: ['Color', 'Mirror', 'Feedback', 'Kaleido', 'Glitch'] },
  { category: 'Outputs', items: ['Display 2', 'Syphon', 'NDI', 'Record'] },
];

/* ---------- Inspector (right column) ---------- */

export interface PatchInspectorSource {
  readonly name: string;
  readonly icon: 'midi' | 'link';
  readonly tint: string;
}

export interface PatchInspectorRow {
  readonly key: string;
  readonly value: number;
  readonly unit: string;
  readonly source: PatchInspectorSource | null;
}

/** Mirrors the bundle's inspector array under "Inspector · Color Processor". */
export const PATCH_INSPECTOR_TITLE = 'Inspector · Color Processor';

export const PATCH_INSPECTOR_ROWS: readonly PatchInspectorRow[] = [
  {
    key: 'hue',
    value: 137,
    unit: '°',
    source: { name: 'Audio · mid', icon: 'link', tint: 'var(--color-cat-color)' },
  },
  {
    key: 'saturation',
    value: 0.75,
    unit: '',
    source: { name: 'Scene · speed h', icon: 'link', tint: 'var(--color-cat-limit)' },
  },
  {
    key: 'brightness',
    value: 0.5,
    unit: '',
    source: { name: 'MIDI · CC 14', icon: 'midi', tint: 'var(--color-primary)' },
  },
  { key: 'contrast', value: 0.55, unit: '', source: null },
];

/**
 * Per-node inspector rows. Selecting a node in the W31 port swaps
 * the inspector body to the entry below; nodes without an explicit
 * entry fall back to a generated row-per-pin shape via
 * {@link inspectorRowsForNode}. Only `color` ships with the bundle's
 * editorial Color-Processor rows; all other nodes are pin-name fallbacks.
 */
export const PATCH_INSPECTOR_BY_NODE: Readonly<Record<string, readonly PatchInspectorRow[]>> = {
  color: PATCH_INSPECTOR_ROWS,
};

/**
 * Resolve the inspector rows for a given node id. Returns the
 * editorial rows from {@link PATCH_INSPECTOR_BY_NODE} when present,
 * otherwise generates fallback rows from the node's pin list with
 * `value: 0`, no unit, and no source. Returns the bundle's
 * Color-Processor rows when `nodeId` is `null` (no selection) so the
 * inspector renders a meaningful default frame on first paint.
 */
export function inspectorRowsForNode(
  nodeId: string | null,
  nodes: readonly { readonly id: string; readonly pins: readonly string[] }[],
): readonly PatchInspectorRow[] {
  if (nodeId === null) return PATCH_INSPECTOR_ROWS;
  const explicit = PATCH_INSPECTOR_BY_NODE[nodeId];
  if (explicit !== undefined) return explicit;
  const node = nodes.find((n) => n.id === nodeId);
  if (node === undefined) return [];
  return node.pins.map((pin) => ({ key: pin, value: 0, unit: '', source: null }));
}

/**
 * Resolve the inspector title for a given node id. Mirrors the
 * bundle's "Inspector · {title}" pattern; falls back to the bundle's
 * Color-Processor title when no node is selected.
 */
export function inspectorTitleForNode(
  nodeId: string | null,
  nodes: readonly { readonly id: string; readonly title: string }[],
): string {
  if (nodeId === null) return PATCH_INSPECTOR_TITLE;
  const node = nodes.find((n) => n.id === nodeId);
  if (node === undefined) return PATCH_INSPECTOR_TITLE;
  return `Inspector · ${node.title}`;
}

/* ---------- Pin geometry helper ---------- */

/**
 * Resolve a pin's `[x, y]` anchor in patch-canvas coordinates. Mirrors
 * the bundle's `getPinPos` helper. Returns `[0, 0]` if the node is
 * unknown so wire rendering degrades gracefully on bad fixtures.
 */
export function patchPinPos(
  nodeId: string,
  pinName: string,
  side: 'in' | 'out',
): readonly [number, number] {
  return patchPinPosFrom(PATCH_NODES, nodeId, pinName, side);
}

/**
 * Variant of {@link patchPinPos} that resolves against a caller-
 * supplied node array — used by the W31 port to compute wire
 * endpoints from the live mutable `$state` nodes (so wires re-route
 * automatically when nodes are dragged). Same `[0, 0]` graceful-
 * degradation contract as the canonical helper.
 */
export function patchPinPosFrom(
  nodes: readonly {
    readonly id: string;
    readonly x: number;
    readonly y: number;
    readonly w: number;
    readonly pins: readonly string[];
  }[],
  nodeId: string,
  pinName: string,
  side: 'in' | 'out',
): readonly [number, number] {
  const node = nodes.find((n) => n.id === nodeId);
  if (!node) return [0, 0];
  const pinIdx = node.pins.indexOf(pinName);
  if (pinIdx === -1) return [0, 0];
  const y = node.y + PATCH_HEADER_H + pinIdx * PATCH_ROW_H + PATCH_ROW_H / 2;
  const x = side === 'out' ? node.x + node.w : node.x;
  return [x, y];
}
