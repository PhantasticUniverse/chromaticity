// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 6 PR 2 audio-uniform IPC consumer store. In-memory `$state`-
// backed singleton subscribing to the Tauri `audio_uniform_snapshot_changed`
// event emitted by the producer-side scheduler tick (Wave 6 PR 1, merge
// `c1d93e8`) per `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` §Decision
// §1 (push event) and §7 (opt-in listener semantics).
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// W33b `capabilities.svelte.ts` sibling is the live precedent for this
// shape (class-instance singleton + module-level export).
//
// **Phase posture.** The Tauri-shell-side concrete `IpcEmit` adapter +
// scheduler-tick bootstrap + audio-thread bootstrap that would actually
// fire `audio_uniform_snapshot_changed` events in production are NOT
// part of Wave 6 PR 2 — they will be addressed in a future "production
// audio bootstrap" wave. Until then the store stays at `null` and the
// visualizer widgets fall back to their existing synthetic time-domain
// generators (per the dual-mode framing consistent with ADR-019
// §Decision §6 stale-but-last-good error model — when no producer
// signal exists, the consumer surface degrades gracefully).
//
// `subscribe()` is idempotent so widgets can call it on every mount
// without leaking listener registrations. When the import or `listen`
// call throws (running outside Tauri, e.g. `npm run dev`'s vite-only
// mode, or Vitest with no IPC mock), the failure is swallowed and the
// store stays `null` — the same fallback path as "no events fire yet".

import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';

/** Tauri unlisten handle returned by `listen()`. Stored so `unsubscribe()` can detach. */
type UnlistenFn = () => void;

class AudioSnapshotStore {
  /** Latest snapshot from the engine, or `null` if no event has arrived yet. */
  snapshot: AudioUniformSnapshotPayload | null = $state(null);

  /** In-flight or settled subscribe promise (idempotency latch). */
  #subscribePromise: Promise<void> | null = null;

  /** Detach handle returned by `listen()`. */
  #unlisten: UnlistenFn | null = null;

  /**
   * Subscribe to the `audio_uniform_snapshot_changed` Tauri event.
   * Idempotent — first call attaches; subsequent calls return the same
   * settled promise so widget mount-time `$effect`s can call this
   * unconditionally without leaking listener registrations.
   */
  subscribe(): Promise<void> {
    if (this.#subscribePromise) return this.#subscribePromise;
    this.#subscribePromise = this.#doSubscribe();
    return this.#subscribePromise;
  }

  async #doSubscribe(): Promise<void> {
    try {
      // Dynamic import + try/catch: when running outside Tauri (vite dev
      // server with no shell, Vitest jsdom with no IPC mock) the import
      // or the call may reject. Treat that as "no events will arrive";
      // widgets fall back to synthetic generators.
      const { listen } = await import('@tauri-apps/api/event');
      const handle = await listen<AudioUniformSnapshotPayload>(
        'audio_uniform_snapshot_changed',
        (event) => {
          this.snapshot = event.payload;
        },
      );
      this.#unlisten = handle;
    } catch {
      // Swallow — no Tauri runtime available; snapshot stays `null` and
      // the dual-mode widget fallback path keeps rendering synthetic.
    }
  }

  /**
   * Detach the listener (test harness; production lifetime is
   * app-lifetime so `unsubscribe()` is not called from app code).
   */
  unsubscribe(): void {
    if (this.#unlisten) {
      this.#unlisten();
      this.#unlisten = null;
    }
    this.#subscribePromise = null;
    this.snapshot = null;
  }
}

export const audioSnapshotStore = new AudioSnapshotStore();
