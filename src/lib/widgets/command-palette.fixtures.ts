// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-29 PR1 CommandPalette fixture data + action type. Lives outside
// the `CommandPalette.svelte` component because Svelte's `*.svelte` TS
// shim only re-exports the default export — named exports inside
// `<script module>` are unreachable to consumers. Same workaround as
// `snap-bar.fixtures.ts` (W27 PR1).

/** One palette row. `id` is a stable string used by the AppShell mount
 *  to attach `onSelect` route-dispatch stubs to the two routable rows
 *  (`launch-scene`, `open-hotkeys`) without coupling the fixture to
 *  `routeStore`. `meta` is the right-aligned context annotation
 *  rendered in mono next to the label. */
export interface CommandPaletteAction {
  readonly id: string;
  readonly label: string;
  readonly meta: string;
  readonly onSelect?: () => void;
}

/** Bundle-faithful default action list from `extras.jsx` lines 127-135.
 *  `onSelect` is omitted on rows whose real dispatch is
 *  `EV-INTEGRATION-MATRIX` scope — the palette's close-on-select
 *  behaviour still fires, so the row is dismissable but a no-op until
 *  the integration lands. */
export const DEFAULT_COMMAND_PALETTE_ACTIONS: readonly CommandPaletteAction[] = [
  { id: 'launch-scene', label: 'Launch scene…', meta: 'library · Enter' },
  { id: 'snap-recall', label: 'Recall Snap slot', meta: 'perform · /favslots/{n}' },
  { id: 'snap-save', label: 'Save current state to Snap', meta: 'perform · /favslots/{n}/save' },
  { id: 'osc-address', label: 'Show OSC address', meta: 'mapping · tooltip' },
  { id: 'open-hotkeys', label: 'Open Settings → Hotkeys', meta: 'help · Cmd+/' },
  {
    id: 'support-bundle',
    label: 'Generate support bundle',
    meta: 'diagnostics · privacy reviewed',
  },
  { id: 'offline-docs', label: 'Open offline docs', meta: 'help · no network' },
];
