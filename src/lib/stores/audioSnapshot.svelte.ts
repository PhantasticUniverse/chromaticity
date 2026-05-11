// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Audio-uniform IPC consumer store. In-memory `$state`-backed
// singleton subscribing to the Tauri `audio_uniform_snapshot_changed`
// event emitted by the producer-side scheduler tick per
// `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` §Decision §1 (push
// event) and §7 (opt-in listener semantics).
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// `capabilities.svelte.ts` sibling is the live precedent for this
// shape (class-instance singleton + module-level export).
//
// **Production bootstrap status.** Live in production since Wave
// 1c-α-2: `engine/audio::AudioRuntime` spawns the producer thread
// feeding `Arc<Mutex<AudioUniformSnapshot>>`;
// `app/shell::TauriIpcEmitter` is the concrete `IpcEmit` adapter; the
// drift-corrected 60 Hz scheduler-tick thread in
// `app/shell/src/lib.rs::run` pulls from `MutexAudioPull` and emits via
// `app.emit("audio_uniform_snapshot_changed", ...)`. When `npm run
// tauri build` produces a binary on a host with a default cpal input
// device (microphone), this store receives live snapshots on every
// tick. On hosts without a default input device (CI / headless /
// device-less), the producer falls back to `SilentSource` and the
// snapshot stays at the all-zero default — visualizer widgets keep
// their dual-mode synthetic fallback for that case (per ADR-019
// §Decision §6 stale-but-last-good).
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
