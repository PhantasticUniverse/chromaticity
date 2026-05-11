// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.4 playlist IPC consumer store. In-memory `$state`-
// backed singleton wrapping the `advance_playlist` + `playlist_position`
// Tauri commands shipped by `app/shell/src/commands/playlist.rs`.
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
// import or the `invoke()` call may reject. The store swallows that
// failure and leaves `position` at its previous value, mirroring the
// `loadedScene.svelte.ts` precedent.
//
// **Engine semantics.** The engine's
// `chromaticity_runtime::Command::PlaylistAdvance` carries a single
// `step: i32` field; the cursor saturates at `i64::MAX` / `i64::MIN`
// per `engine/runtime/src/state.rs`. There is no engine-side
// shuffle / jump-to-index command — the bundle's "shuffle" button
// surfaces as a single forward step here, with the random-step
// follow-on deferred to a future wave (the engine would need a new
// `Command::PlaylistShuffle` or a playlist-length read so the
// frontend can pick a uniform random offset).

class PlaylistStore {
  /** Engine cursor position, defaulting to 0 before any advance. */
  position: number = $state(0);

  /**
   * Read the engine's current playlist cursor and update `position`.
   * Used for mount-time hydration and as the post-commit read-back
   * step inside `advance()`.
   *
   * No-Tauri-context behaviour: when the `@tauri-apps/api/core`
   * import fails (vite dev / vitest), the call resolves without
   * touching `position` — same fallback path as
   * `loadedScene.svelte.ts::hydrate`.
   */
  async refresh(): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const pos = await invoke<number>('playlist_position');
      this.position = pos;
    } catch {
      // Swallow — no Tauri runtime available; `position` stays at
      // its previous value (typically `0` at startup).
    }
  }

  /**
   * Dispatch an `advance_playlist` for `step` and then refresh
   * `position`. Negative `step` moves backward (engine saturates at
   * `i64::MIN`). Resolves once the round-trip completes.
   */
  async advance(step: number): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('advance_playlist', { step });
      await this.refresh();
    } catch {
      // Swallow — no Tauri runtime available; `position` stays at
      // its previous value.
    }
  }

  /** Convenience wrapper: advance by +1. */
  async next(): Promise<void> {
    await this.advance(1);
  }

  /** Convenience wrapper: advance by -1. */
  async previous(): Promise<void> {
    await this.advance(-1);
  }

  /** Reset the store to the pristine cursor baseline (test harness). */
  reset(): void {
    this.position = 0;
  }
}

export const playlistStore = new PlaylistStore();
