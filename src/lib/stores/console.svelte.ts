// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-21 PR4 global console drawer store. App-level singleton that
// owns the drawer's open / paused flags and the active filter spread.
// Wired by `StatusBar.svelte`'s Console button (toggle), `AppShell.svelte`
// (mounts `<ConsoleDrawer />` overlay), and the drawer itself
// (filter mutations + close-on-escape / backdrop).
//
// `.svelte.ts` extension lets the Svelte 5 compiler process `$state`
// outside a `.svelte` component — the `routeStore` precedent. The
// class-instance pattern keeps reactive reads shallow (no subscription
// glue) and gives `$derived` consumers a single import surface.
//
// Locked design choices per the W21 PR4 brief:
//   - filter chips AND-combined with source dropdown AND search query
//   - severity union mirrors the bundle's `import.jsx` console levels
//     plus the W20 feature-parity addition of `error` (Import view's
//     `err` is the four-letter spelling there; the global drawer
//     standardises on `error` to match the StatusBar's `errors` counter
//     copy and the `--color-signal-red` semantic naming).
//   - source filter is a `RouteName` string union plus the literal
//     `'all'` sentinel — wider than just the active route since logs
//     accumulate from background subscribers across views.
//   - Real log-stream subscription is Phase-3+/4a `EV-OBSERVABILITY`
//     scope; this slice ships the toggle-only stub and the consumer
//     wires fixture entries from `data/console-fixtures.ts`.

import { SvelteSet } from 'svelte/reactivity';

import type { ConsoleEntry } from '../data/console-fixtures';
import { overlayStack, registerStateMachine } from '../state';
import type { RouteName } from './route.svelte';

/** Console-entry severity levels. Drives the row's left-border tint
 *  via the `--color-signal-*` token cascade. The seven-level vocabulary
 *  mirrors the v0.2 Import-view console (`scan` / `load` / `fetch` /
 *  `compile` / `warn`) and adds `error` for the StatusBar's errors
 *  counter alignment. `info` is reserved for future operator-prompt
 *  rows (e.g. snap recall echoes); not surfaced as a chip in W21 PR4. */
export type ConsoleSeverity = 'scan' | 'load' | 'fetch' | 'compile' | 'warn' | 'error' | 'info';

/** Source filter — `'all'` plus the routed views from the W19 PR2
 *  full-port surface. `scene-import` is a synthetic source for engine
 *  log lines that originated from a scene-import worker rather than
 *  from a routed view; it is NOT a `RouteName`. */
export type ConsoleSource = 'all' | RouteName | 'scene-import';

/** All severity levels surfaced as chips. `info` is intentionally
 *  excluded (reserved for non-toggleable echo rows). */
export const SEVERITY_CHIPS: readonly ConsoleSeverity[] = [
  'scan',
  'load',
  'fetch',
  'compile',
  'warn',
  'error',
];

class ConsoleStore {
  /** Drawer visibility — backed by the global `overlayStack` (`'console'`). */
  get isOpen(): boolean {
    return overlayStack.isOpen('console');
  }

  set isOpen(value: boolean) {
    if (value) {
      overlayStack.open('console');
    } else {
      overlayStack.close('console');
    }
  }

  /** Live-tail pause flag. The drawer renders the same fixture corpus
   *  regardless; this gates a "paused" visual badge in the action row.
   *  Real subscription pause is Phase-3+/4a scope. */
  isPaused = $state(false);

  /** Active severity filter — `SvelteSet` of toggleable chips. Empty
   *  set means "All" (the inverse of the bundle JSX, but cleaner to
   *  model in the drawer's chip-toggle UI: an empty filter is the All
   *  chip's pressed state). `SvelteSet` is the reactive variant from
   *  `svelte/reactivity` that emits on `add`/`delete` without manual
   *  reassignment — the lint rule `svelte/prefer-svelte-reactivity`
   *  forbids the plain `Set` here. */
  severityFilter = new SvelteSet<ConsoleSeverity>();

  /** Active source filter. `'all'` is the default sentinel. */
  sourceFilter = $state<ConsoleSource>('all');

  /** Free-text query — case-insensitive substring match against the
   *  message body and source label. */
  query = $state('');

  /** Live runtime diagnostic log entries. Starts empty so a healthy
   *  session reports 0 warnings and 0 errors. */
  entries = $state<ConsoleEntry[]>([]);

  get warningsCount(): number {
    return this.entries.filter((entry) => entry.severity === 'warn').length;
  }

  get errorsCount(): number {
    return this.entries.filter((entry) => entry.severity === 'error').length;
  }

  push(entry: ConsoleEntry): void {
    this.entries = [...this.entries, entry];
  }

  clearEntries(): void {
    this.entries = [];
  }

  toggle(): void {
    overlayStack.toggle('console');
  }

  open(): void {
    overlayStack.open('console');
  }

  close(): void {
    overlayStack.close('console');
  }

  togglePause(): void {
    this.isPaused = !this.isPaused;
  }

  toggleSeverity(severity: ConsoleSeverity): void {
    if (this.severityFilter.has(severity)) {
      this.severityFilter.delete(severity);
    } else {
      this.severityFilter.add(severity);
    }
  }

  setSource(source: ConsoleSource): void {
    this.sourceFilter = source;
  }

  setQuery(value: string): void {
    this.query = value;
  }

  clearFilters(): void {
    this.severityFilter.clear();
    this.sourceFilter = 'all';
    this.query = '';
  }
}

export const consoleStore = new ConsoleStore();

registerStateMachine('console', () => ({
  isOpen: consoleStore.isOpen,
  isPaused: consoleStore.isPaused,
  sourceFilter: consoleStore.sourceFilter,
}));
