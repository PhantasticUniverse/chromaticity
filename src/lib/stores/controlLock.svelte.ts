// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
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
// suppressed value. Scene-namespace locks (ids below
// `META_CONTROL_ID_BASE`) are wiped via `syncSceneScope` whenever the
// loaded scene changes — lock ids are bare schema positions shared
// across scenes, so they must not leak onto the next scene's rows.
// Meta-namespace locks survive scene loads (mirroring the value
// cache's cross-scene `meta:` namespace) and are wiped on `reset()`
// or page reload (no localStorage persistence — pre-alpha posture;
// future waves may persist).
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

  /** Scene id the current scene-namespace locks belong to — plain
   * field, intentionally non-reactive bookkeeping for
   * {@link syncSceneScope}. Lives in the store (not the Perform
   * component) so the comparison survives the AppShell `{#if}`-ladder
   * remounts: a scene loaded from another view while Perform was
   * unmounted still invalidates the stale locks on the next sync. */
  private sceneScope: string | null = null;

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

  /**
   * Report the active scene id; when it differs from the recorded
   * scope, every lock id below `boundary` (pass
   * `META_CONTROL_ID_BASE`) is unlocked. Idempotent while the scene
   * stays the same, so callers can invoke it from a tracking
   * `$effect` on every run. The `null` (no scene / fixture) scope is
   * a distinct namespace: fixture-row locks clear when a real scene
   * loads, and vice versa.
   */
  syncSceneScope(sceneId: string | null, boundary: number): void {
    if (sceneId === this.sceneScope) return;
    this.sceneScope = sceneId;
    for (const id of [...this.locked]) {
      if (id < boundary) this.locked.delete(id);
    }
  }

  /** Reset the store to the no-locks baseline (test harness). */
  reset(): void {
    this.locked.clear();
    this.sceneScope = null;
  }
}

export const controlLockStore = new ControlLockStore();
