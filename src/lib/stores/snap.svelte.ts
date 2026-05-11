// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.3 Snap IPC consumer store. In-memory `$state`-backed
// singleton wrapping the `recall_snap` / `save_snap` / `clear_snap` /
// `active_snap_slot` / `snap_slot_saved` Tauri commands exposed by
// `app/shell/src/commands/snap.rs`.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// `audioSnapshot.svelte.ts` + `loadedScene.svelte.ts` siblings are the
// live precedents for this shape (class-instance singleton +
// module-level export).
//
// **SnapBar domain.** The SnapBar widget renders 8 slots indexed 1..8;
// the engine accepts u8 slot ids in `0..=15` (`SnapSlotId::MAX = 15`).
// The store exposes a `SnapBar.SLOT_COUNT = 8` surface and maps the
// 1-indexed widget slot id to the 0-indexed engine slot via `slot - 1`
// inside each method. Callers always pass the widget-facing 1..8 ids.
//
// **Dual-mode posture (no-Tauri-context fallback).** When the Tauri
// IPC module is unavailable — `npm run dev`'s vite-only mode, Vitest
// jsdom with no IPC mock — the dynamic import or `invoke()` call may
// reject. The store swallows that failure and leaves state at the
// no-snap baseline (`activeSlot = null`, `saved = [false × 8]`),
// mirroring the `audioSnapshot.svelte.ts` Wave 1c-α precedent so the
// SnapBar widget keeps rendering with the empty visual state.

/** Number of SnapBar slots exposed to the widget. The engine surface
 *  is 16 slots (`SnapSlotId::MAX = 15`); the SnapBar surface trims that
 *  to the first 8 per the bundle's `extras.jsx` fixture. */
export const SNAP_SLOT_COUNT = 8;

class SnapStore {
  /** Active SnapBar slot (1-indexed), or `null` when none is recalled. */
  activeSlot: number | null = $state(null);

  /** Per-slot saved-state mask indexed by `slot - 1` (i.e. saved[0]
   *  is the saved state for SnapBar slot 1). Length always equals
   *  `SNAP_SLOT_COUNT`. */
  saved: boolean[] = $state(Array.from({ length: SNAP_SLOT_COUNT }, () => false));

  /** Most recent error message from a failed `recall`/`save`/`clear`/
   *  `refresh` call, or `null` on success. */
  error: string | null = $state(null);

  /**
   * Hydrate `activeSlot` + `saved[]` from the engine by invoking
   * `active_snap_slot` once and `snap_slot_saved` once per slot.
   * Idempotent — callers can call this on every widget mount.
   *
   * No-Tauri-context behaviour: the dynamic import or `invoke()` may
   * reject; the failure is swallowed and state stays at the no-snap
   * baseline (active=null, saved=all-false). Mirrors the
   * `audioSnapshot.svelte.ts::subscribe` and
   * `loadedScene.svelte.ts::hydrate` fallback paths.
   */
  async refresh(): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const engineActive = await invoke<number | null>('active_snap_slot');
      const savedQueries: Promise<boolean>[] = [];
      for (let engineSlot = 0; engineSlot < SNAP_SLOT_COUNT; engineSlot += 1) {
        savedQueries.push(invoke<boolean>('snap_slot_saved', { slot: engineSlot }));
      }
      const saved = await Promise.all(savedQueries);
      // Map engine slot 0..7 back to SnapBar slot 1..8 for `activeSlot`.
      this.activeSlot =
        engineActive !== null && engineActive >= 0 && engineActive < SNAP_SLOT_COUNT
          ? engineActive + 1
          : null;
      this.saved = saved;
      this.error = null;
    } catch {
      // Swallow — no Tauri runtime available; state stays at the
      // no-snap baseline so the SnapBar widget renders empty.
    }
  }

  /**
   * Dispatch a `recall_snap` for the SnapBar `slot` (1-indexed) and
   * update `activeSlot` to point at that slot on success. Rejection
   * is captured in `error` and `activeSlot` stays unchanged.
   */
  async recall(slot: number): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('recall_snap', { slot: slot - 1 });
      this.activeSlot = slot;
      this.error = null;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /**
   * Dispatch a `save_snap` for the SnapBar `slot` (1-indexed) and mark
   * the slot as saved on success.
   */
  async save(slot: number): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('save_snap', { slot: slot - 1 });
      const next = [...this.saved];
      const idx = slot - 1;
      if (idx >= 0 && idx < next.length) {
        next[idx] = true;
      }
      this.saved = next;
      this.error = null;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /**
   * Dispatch a `clear_snap` for the SnapBar `slot` (1-indexed) and
   * mark the slot as empty on success; if the cleared slot was the
   * active one, `activeSlot` flips to `null`.
   */
  async clear(slot: number): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('clear_snap', { slot: slot - 1 });
      const next = [...this.saved];
      const idx = slot - 1;
      if (idx >= 0 && idx < next.length) {
        next[idx] = false;
      }
      this.saved = next;
      if (this.activeSlot === slot) {
        this.activeSlot = null;
      }
      this.error = null;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Reset the store to the no-snap baseline (test harness). */
  reset(): void {
    this.activeSlot = null;
    this.saved = Array.from({ length: SNAP_SLOT_COUNT }, () => false);
    this.error = null;
  }
}

export const snapStore = new SnapStore();
