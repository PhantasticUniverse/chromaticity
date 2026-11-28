/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * Operator hotkey map — TypeScript mirror of the predev bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/hotkeys_data.jsx`,
 * byte-equivalent groupings + labels per the W8 plan agent's verification.
 *
 * Upstream normative spec: `docs/42-operator-hotkey-map.md`. The bundle is
 * an archive-only mirror; this constant is the production-side source for
 * the `?`-toggle overlay (`src/lib/HotkeyOverlay.svelte`) and any future
 * Settings → Hotkeys read-only table. Pure data, no logic.
 *
 * V1 posture per `docs/42` §V1 rebinding: rows are read-only; rebinding
 * ships in V2. The `cap` column surfaces entitlement gating for the
 * a11y/entitlement sweep.
 */

export interface HotkeyRow {
  readonly keys: readonly string[];
  readonly action: string;
  readonly osc?: string;
  readonly glsl?: string;
  readonly cap?: string;
}

export interface HotkeyGroup {
  readonly title: string;
  readonly note?: string;
  readonly rows: readonly HotkeyRow[];
}

export const HOTKEYS: readonly HotkeyGroup[] = [
  {
    title: 'Control-focused (D / R / P / L)',
    note: 'Active only while a single control has focus. `Esc` exits control-focused mode.',
    rows: [
      { keys: ['D'], action: 'Set control to its default value' },
      { keys: ['R'], action: 'Set control to a random value in its range' },
      { keys: ['P'], action: 'Set control to its value in the current preset' },
      { keys: ['L'], action: "Toggle the control's lock state (ignore randomization)" },
    ],
  },
  {
    title: 'Scene & playlist transport',
    rows: [
      {
        keys: ['Space'],
        action: 'Toggle playlist play/pause',
        osc: '/playlist/toggleplay',
        cap: 'free',
      },
      { keys: ['→', 'K'], action: 'Next scene in playlist', osc: '/playlist/next', cap: 'free' },
      {
        keys: ['←', 'J'],
        action: 'Previous scene in playlist',
        osc: '/playlist/previous',
        cap: 'free',
      },
      {
        keys: ['Alt+Cmd+→', 'Ctrl+Alt+→'],
        action: 'Jump to last in playlist',
        osc: '/playlists/position -1',
        cap: 'free',
      },
      {
        keys: ['Alt+Cmd+←', 'Ctrl+Alt+←'],
        action: 'Jump to first in playlist',
        osc: '/playlists/position 0',
        cap: 'free',
      },
      {
        keys: ['Enter'],
        action: 'Launch selected scene from the library',
        osc: '/scenes/{name}',
        cap: 'free',
      },
      {
        keys: ['Shift+Enter'],
        action: 'Launch selected scene with preset prompt',
        osc: '/scenes/{name} {preset}',
        cap: 'free',
      },
      {
        keys: ['Cmd+Enter', 'Ctrl+Enter'],
        action: 'Launch selected scene into a playlist slot',
        osc: '(internal)',
        cap: 'free',
      },
    ],
  },
  {
    title: 'Snap slots',
    note: 'Per-scene quick-recall preset slots. Wire literal `favslot` retained under compat-syn-v1.',
    rows: [
      {
        keys: ['1 – 9', '0'],
        action: 'Recall Snap slot 1–10 for the current scene',
        osc: '/favslots/{n} (apply)',
        cap: 'free',
      },
      {
        keys: ['Shift+1 – Shift+0'],
        action: 'Save current state to Snap slot 1–10 (overwrite without prompt)',
        osc: '/favslots/{n}/save',
        cap: 'free',
      },
      {
        keys: ['Alt+Shift+1 – 0', 'Ctrl+Alt+Shift+1 – 0'],
        action: 'Clear Snap slot 1–10 (confirmation prompt)',
        osc: '/favslots/{n}/clear',
        cap: 'free',
      },
      {
        keys: ['Cmd+S', 'Ctrl+S'],
        action: 'Save current state to the next empty Snap slot',
        osc: '/favslots/{auto}/save',
        cap: 'free',
      },
      {
        keys: ['Cmd+Opt+S', 'Ctrl+Alt+S'],
        action: 'Save as a new named preset (prompts)',
        osc: '/presets/new {name}',
        cap: 'free',
      },
      {
        keys: ['Cmd+0', 'Ctrl+0'],
        action: 'Recall the scene’s “gold” preset (Snap 0)',
        osc: '/favslots/0 (apply)',
        cap: 'free',
      },
    ],
  },
  {
    title: 'Display / output',
    rows: [
      {
        keys: ['F'],
        action: 'Toggle fullscreen on the primary output',
        osc: '(internal)',
        cap: 'free',
      },
      {
        keys: ['Shift+F'],
        action: 'Toggle fullscreen on the secondary output (multi-display)',
        osc: '(internal)',
        cap: 'free',
      },
      {
        keys: ['Cmd+Shift+F', 'Ctrl+Shift+F'],
        action: 'Cycle output window placement across displays',
        osc: '(internal)',
        cap: 'free',
      },
      {
        keys: ['Cmd+\\', 'Ctrl+\\'],
        action: 'Swap primary and secondary output',
        osc: '(internal)',
        cap: 'free',
      },
    ],
  },
  {
    title: 'Rendering',
    note: 'Cmd+0 is globally bound to Snap 0; the former “reset resolution” binding has moved to Cmd+9.',
    rows: [
      {
        keys: ['.'],
        action: 'Toggle render loop on/off',
        osc: '/render/enabled {0|1}',
        cap: 'free',
      },
      { keys: [','], action: 'Toggle render-preview window', osc: '(internal)', cap: 'free' },
      {
        keys: ['Cmd+=', 'Ctrl+='],
        action: 'Step render resolution up',
        osc: '/render/resolution {w} {h}',
        cap: 'render.custom-resolution',
      },
      {
        keys: ['Cmd+-', 'Ctrl+-'],
        action: 'Step render resolution down',
        osc: '/render/resolution {w} {h}',
        cap: 'render.custom-resolution',
      },
      {
        keys: ['Cmd+9', 'Ctrl+9'],
        action: 'Reset render resolution to scene default',
        osc: '/render/resolution {sceneW} {sceneH}',
        cap: 'render.custom-resolution',
      },
      {
        keys: ['Cmd+Shift+=', 'Ctrl+Shift+='],
        action: 'Open FPS target dialog',
        osc: '/render/fps {float}',
        cap: 'free',
      },
    ],
  },
  {
    title: 'Meta controls',
    note: 'Each row enters a modal slider-adjust mode (arrow keys or scroll wheel). `Esc` exits.',
    rows: [
      { keys: ['M'], action: 'Master', glsl: '_Master' },
      { keys: ['B'], action: 'Toggle Blackout (global panic)', glsl: '_Blackout' },
      { keys: ['G'], action: 'Gamma', glsl: '_Gamma' },
      { keys: ['C'], action: 'Contrast', glsl: '_Contrast' },
      { keys: ['H'], action: 'Hue', glsl: '_Hue' },
      { keys: ['Y'], action: 'Saturation', glsl: '_Saturation' },
      { keys: ['I'], action: 'Invert toggle (no modal; binary flip)', glsl: '_Invert' },
      { keys: ['V'], action: 'Vertical mirror toggle', glsl: '_Vert_Mirror' },
      { keys: ['Shift+V'], action: 'Horizontal mirror toggle', glsl: '_Hor_Mirror' },
      { keys: ['E'], action: 'Dichrome toggle (from “Edition”)', glsl: '_Limit_Colors' },
    ],
  },
  {
    title: 'Audio',
    rows: [
      { keys: ['A'], action: 'Toggle audio-input mute', osc: '(internal)', cap: 'free' },
      {
        keys: ['Cmd+A', 'Ctrl+A'],
        action: 'Open audio settings panel',
        osc: '(internal)',
        cap: 'free',
      },
      {
        keys: ['T'],
        action: 'Toggle audio-threshold visualizer overlay',
        osc: '(internal)',
        cap: 'free',
      },
      {
        keys: ['Shift+T'],
        action: 'Set audio threshold to current RMS level',
        osc: '/audio/threshold/sample',
        cap: 'free',
      },
    ],
  },
  {
    title: 'Navigation panels',
    rows: [
      {
        keys: ['Cmd+K', 'Ctrl+K'],
        action: 'Open command palette (searches scenes, presets, settings, help)',
      },
      { keys: ['Cmd+/', 'Ctrl+/'], action: 'Open help / docs search' },
      { keys: ['Cmd+L', 'Ctrl+L'], action: 'Open scene library' },
      { keys: ['Cmd+,', 'Ctrl+,'], action: 'Open settings' },
      { keys: ['Cmd+O', 'Ctrl+O'], action: 'Open OSC / MIDI mapping panel' },
    ],
  },
  {
    title: 'Diagnostics & debug',
    rows: [
      { keys: ['Alt+Cmd+D', 'Ctrl+Alt+D'], action: 'Toggle diagnostics console' },
      { keys: ['Alt+Cmd+P', 'Ctrl+Alt+P'], action: 'Toggle performance HUD' },
      { keys: ['Alt+Cmd+L', 'Ctrl+Alt+L'], action: 'Toggle OSC/MIDI learn indicator' },
      { keys: ['Cmd+R', 'Ctrl+R'], action: 'Reload current scene' },
      { keys: ['Alt+Cmd+R', 'Ctrl+Alt+R'], action: 'Reset meta controls to defaults' },
    ],
  },
  {
    title: 'Show state',
    rows: [
      {
        keys: ['Cmd+Shift+N', 'Ctrl+Shift+N'],
        action: 'New show (confirmation if unsaved changes)',
      },
      { keys: ['Cmd+Shift+O', 'Ctrl+Shift+O'], action: 'Open .chromaShow bundle' },
      { keys: ['Cmd+Shift+S', 'Ctrl+Shift+S'], action: 'Save show as .chromaShow bundle' },
      {
        keys: ['Cmd+Z', 'Ctrl+Z'],
        action: 'Undo (operator-level; one level for preset recall, meta tweak, scene switch)',
      },
      { keys: ['Cmd+Shift+Z', 'Ctrl+Shift+Z'], action: 'Redo' },
    ],
  },
];
