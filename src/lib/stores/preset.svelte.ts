// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.4 preset IPC consumer store. In-memory `$state`-
// backed singleton wrapping the `recall_preset` + `active_preset`
// Tauri commands shipped by `app/shell/src/commands/preset.rs`.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the Wave 2.1 Lane A.1 `loadedScene.svelte.ts` + Wave 1c-α
// `audioSnapshot.svelte.ts` siblings are the live precedents for this
// shape (class-instance singleton + module-level export).
//
// **Dual-mode posture (no-Tauri-context fallback).** When the Tauri
// IPC module is unavailable — e.g. running under `npm run dev`'s
// vite-only mode, or Vitest jsdom with no IPC mock — the dynamic
// import rejects. The store swallows that failure and leaves
// `activeId` at its previous value, mirroring the
// `loadedScene.svelte.ts` precedent so consumers degrade gracefully
// to a fixture-driven fallback. A rejection from `invoke()` itself
// is a real engine failure — the backing commands return
// `Result<_, String>` (state mutex poison, `CommandBusError`) — and
// is recorded into `error` per the `snap.svelte.ts` /
// `settings.svelte.ts` error-surface idiom instead of being
// swallowed silently.
//
// The plain-`.ts` sibling `preset.ts` exposes
// `dispatchPresetRecallAndRead` and `readActivePreset` — request /
// response helpers without reactive state. This store layers reactive
// `$state` on top of those helpers (re-implemented via dynamic
// import here to keep the no-Tauri-context fallback uniform across
// the two methods) so consumers can read `presetStore.activeId`
// without subscribing to a promise.

import type { PresetId } from '../ipc/bindings';

class PresetStore {
  /** Active preset id reported by the engine, or `null` when none recalled. */
  activeId: PresetId | null = $state(null);

  /** Most recent error message from a failed `refresh`/`recall`/
   *  `save` invoke round-trip, or `null` on success (cleared by the
   *  next successful call). No-Tauri-runtime import failures do NOT
   *  populate this field — see the dual-mode posture above. Mirrors
   *  `snap.svelte.ts::error`. */
  error: string | null = $state(null);

  /**
   * Read the engine's current active preset and update `activeId`.
   * Used for mount-time hydration and as the post-commit read-back
   * step inside `recall()`.
   *
   * No-Tauri-context behaviour: when the `@tauri-apps/api/core`
   * import fails (vite dev / vitest), the call resolves without
   * touching `activeId` — same fallback path as
   * `loadedScene.svelte.ts::hydrate`. An `invoke()` rejection is a
   * real engine failure and is recorded into `error`.
   */
  async refresh(): Promise<void> {
    let core: typeof import('@tauri-apps/api/core');
    try {
      core = await import('@tauri-apps/api/core');
    } catch {
      // Swallow — no Tauri runtime available; `activeId` stays at
      // its previous value and consumers fall back to the default
      // (typically `null` at startup).
      return;
    }
    try {
      const id = await core.invoke<PresetId | null>('active_preset');
      this.activeId = id;
      this.error = null;
    } catch (err) {
      // Real engine rejection (`active_preset` returns
      // `Result<_, String>`); record it for the view layer.
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /**
   * Dispatch a `recall_preset` for `id` and then refresh `activeId`
   * via `active_preset`. Resolves once the round-trip completes.
   *
   * The post-commit refresh mirrors the `dispatchAndRead` shape from
   * the Lane A.2 control wiring: the engine commits the value, the
   * UI reflects the engine-returned value (even if the engine
   * rejected or clamped the input).
   */
  async recall(id: PresetId): Promise<void> {
    let core: typeof import('@tauri-apps/api/core');
    try {
      core = await import('@tauri-apps/api/core');
    } catch {
      // Swallow — no Tauri runtime available; `activeId` stays at
      // its previous value. Vite dev / vitest jsdom both hit this
      // path by design.
      return;
    }
    try {
      await core.invoke<void>('recall_preset', { id });
      await this.refresh();
    } catch (err) {
      // Real engine rejection (state mutex poison, CommandBusError);
      // `activeId` stays at its previous value and the failure is
      // recorded for the view layer. `refresh()` never rejects, so
      // this catch only sees the `recall_preset` failure.
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /**
   * Wave 2.4 PERF.C — save the current control snapshot into preset
   * slot `id`. The engine snapshots `ControlStore.values` and also
   * activates the preset at `State::apply_event` time, so the
   * post-save refresh picks up `id` as the new `activeId` (mirrors
   * the `recall()` post-commit refresh pattern).
   *
   * No-Tauri-context behaviour: same fallback path as `recall()`.
   */
  async save(id: PresetId): Promise<void> {
    let core: typeof import('@tauri-apps/api/core');
    try {
      core = await import('@tauri-apps/api/core');
    } catch {
      // Swallow — no Tauri runtime available; `activeId` stays at
      // its previous value. Vite dev / vitest jsdom both hit this
      // path by design.
      return;
    }
    try {
      await core.invoke<void>('save_preset', { id });
      await this.refresh();
    } catch (err) {
      // Real engine rejection; recorded for the view layer (see
      // `recall()` — same posture).
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Reset the store to the no-preset baseline (test harness). */
  reset(): void {
    this.activeId = null;
    this.error = null;
  }
}

export const presetStore = new PresetStore();
