// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.1 loaded-scene IPC consumer store. In-memory
// `$state`-backed singleton wrapping the `load_scene` + `loaded_scene`
// Tauri commands shipped by `app/shell/src/commands/scene.rs`.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the W33b `capabilities.svelte.ts` + Wave 1c-α `audioSnapshot.svelte.ts`
// siblings are the live precedents for this shape (class-instance
// singleton + module-level export).
//
// **Dual-mode posture (no-Tauri-context fallback).** When the Tauri
// IPC module is unavailable — e.g. running under `npm run dev`'s
// vite-only mode, or Vitest jsdom with no IPC mock — the dynamic
// import or the `invoke()` call may reject. The store swallows that
// failure and leaves `snapshot` at `null`, mirroring the
// `audioSnapshot.svelte.ts` Wave 1c-α precedent so consumers
// degrade gracefully to a fixture-driven fallback.

import type { LoadedScenePayload } from '../ipc/bindings';

class LoadedSceneStore {
  /** Most recently loaded scene payload, or `null` before any load. */
  snapshot: LoadedScenePayload | null = $state(null);

  /** Most recent load-attempt error message, or `null` on success. */
  error: string | null = $state(null);

  /** `true` while a `load()` call is in flight (UI can render spinners). */
  loading: boolean = $state(false);

  /**
   * Load the `.chroma` package at `path` into the runtime and update
   * `snapshot`. Resolves with the new payload; rejects (and updates
   * `error`) when the engine returns a parse failure.
   *
   * No-Tauri-context behaviour: when the `@tauri-apps/api/core`
   * import fails (vite dev / vitest), the call resolves to `null`
   * and `snapshot` stays unchanged — same fallback path as
   * `audioSnapshot.svelte.ts::subscribe`.
   */
  async load(path: string): Promise<LoadedScenePayload | null> {
    this.loading = true;
    this.error = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const payload = await invoke<LoadedScenePayload>('load_scene', { path });
      this.snapshot = payload;
      return payload;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      return null;
    } finally {
      this.loading = false;
    }
  }

  /**
   * Read the most recently loaded scene from the engine and hydrate
   * the store. Used at view mount to re-bind to a scene that was
   * loaded before the component was first rendered (route
   * navigation after a Library double-click).
   */
  async hydrate(): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const payload = await invoke<LoadedScenePayload | null>('loaded_scene');
      if (payload) {
        this.snapshot = payload;
      }
    } catch {
      // Swallow — no Tauri runtime available; snapshot stays whatever
      // it was (likely `null` at startup), and consumers fall back
      // to fixture data.
    }
  }

  /** Reset the store to the no-scene-loaded baseline (test harness). */
  reset(): void {
    this.snapshot = null;
    this.error = null;
    this.loading = false;
  }
}

export const loadedSceneStore = new LoadedSceneStore();
