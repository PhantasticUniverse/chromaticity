// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.4 Lane PERF.A control-lock store. Pure frontend in-memory
// state tracking which control IDs are "locked" — locked controls
// short-circuit `dispatchControl` in `Perform.svelte` so the operator
// can pin select rows to their current value while Random / Default
// or any pointer-drag sweeps the remaining controls.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the `snap.svelte.ts` + `loadedScene.svelte.ts` siblings are the
// live precedents for this shape (class-instance singleton +
// module-level export).
//
// **No IPC seam.** Lock state is purely a UI affordance — the engine
// has no concept of "locked" controls. `dispatchControl` reads
// `controlLockStore.isLocked(id)` and elides the `set_control`
// invoke when the row is locked; the engine never sees the
// suppressed value. Locked-row state is wiped on `reset()` or page
// reload (no localStorage persistence — pre-alpha posture; future
// waves may persist).
//
// **`SvelteSet` over plain `Set`.** The Svelte 5 reactivity
// primitive in `svelte/reactivity` emits change notifications on
// `add` / `delete` without whole-collection reassignment; the lint
// rule `svelte/prefer-svelte-reactivity` forbids the plain `Set` in
// reactive scopes. Matches the `console.svelte.ts::severityFilter`
// and `Library.svelte::activeCategories` precedents.

import { SvelteSet } from 'svelte/reactivity';

class ControlLockStore {
  /** Set of locked control IDs. Membership ↔ locked. */
  locked: SvelteSet<number> = new SvelteSet();

  /** Read-only membership check. */
  isLocked(id: number): boolean {
    return this.locked.has(id);
  }

  /** Toggle lock state for a single control id. */
  toggle(id: number): void {
    if (this.locked.has(id)) {
      this.locked.delete(id);
    } else {
      this.locked.add(id);
    }
  }

  /** Lock every id in `ids` (idempotent — already-locked ids stay locked). */
  lockAll(ids: readonly number[]): void {
    for (const id of ids) {
      this.locked.add(id);
    }
  }

  /** Unlock every id in `ids` (idempotent — already-unlocked ids stay unlocked). */
  unlockAll(ids: readonly number[]): void {
    for (const id of ids) {
      this.locked.delete(id);
    }
  }

  /** Reset the store to the no-locks baseline (test harness + view unmount). */
  reset(): void {
    this.locked.clear();
  }
}

export const controlLockStore = new ControlLockStore();
