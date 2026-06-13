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
// **Production bootstrap status (narrowed).** The producer→IPC→consumer
// transport is live in production since Wave 1c-α-2 and is proven
// end-to-end: `engine/audio::AudioRuntime` spawns the producer thread
// feeding `Arc<Mutex<AudioUniformSnapshot>>`;
// `app/shell::TauriIpcEmitter` is the concrete `IpcEmit` adapter; the
// drift-corrected 60 Hz scheduler-tick thread in
// `app/shell/src/lib.rs::run` pulls from `MutexAudioPull` and emits via
// `app.emit("audio_uniform_snapshot_changed", ...)`. With a working
// capture source (microphone, or an operator-installed virtual loopback
// such as BlackHole/Loopback), this store receives live snapshots on
// every tick — the IPC transport, the analyser, and this consumer were
// verified carrying real, varying values end-to-end (a
// `CHROMATICITY_AUDIO_DIAG` round-trip confirmed the webview receives
// exactly what the engine emits).
//
// **macOS system-audio (built-in) TAP capture is live.** The
// zero-install system-output tap on macOS 14.6+ reads through a direct
// Core Audio `IOProc` (vendored cpal Patch 6, replacing the AUHAL read
// stage that corrupted the stream — see `docs/adr/ADR-022` → Amendment
// 2026-06-03 and `third_party/cpal/UPSTREAM-VERSION.md`), and is
// hardware-verified clean (a 440 Hz / 0.5 sine probe returns raw_peak
// exactly 0.5; AO-confirmed live GUI reactivity 2026-06-05). Below the
// macOS 14.6 floor or on denied `NSAudioCaptureUsageDescription`
// consent, the `system-audio` preference degrades to the default input
// — never silent. On hosts without any input device (CI / headless /
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

  /**
   * Count of `audio_uniform_snapshot_changed` events received since subscribe.
   * Stays `0` if the engine never emits OR the event never reaches the webview
   * — the discriminator (vs a live-but-zero snapshot) for diagnosing a dead
   * IPC seam. Surfaced for diagnostics/observability, not product UI.
   */
  received = $state(0);

  /**
   * Non-null when `subscribe()` failed to attach the listener (Tauri API
   * import or `listen()` threw). Mirrors the `midiLearn` store's error-surface
   * pattern so a dead seam is visible instead of silently masked behind the
   * synthetic fixture fallback. `null` both before subscribe and on success.
   */
  error: string | null = $state(null);

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
          this.received += 1;
          // Diagnostic round-trip (no-op unless CHROMATICITY_AUDIO_DIAG is set
          // on the shell): mirror the received rms/peak back to the engine
          // capture log so the webview-side values appear alongside the
          // producer-side ones — bisecting "static meter despite live engine".
          if (this.received % 60 === 0) {
            void this.#reportRx(event.payload.rms, event.payload.peak, this.received);
          }
        },
      );
      this.#unlisten = handle;
      this.error = null;
    } catch (e) {
      // Record (don't silently swallow) so a dead seam is diagnosable. Outside
      // Tauri (vite dev / Vitest) this is expected and the widget fixture
      // fallback still renders; in a real shell this surfaces a broken bridge.
      this.error = e instanceof Error ? e.message : String(e);
      console.warn('[audioSnapshot] listen() subscribe failed; snapshot stays null:', e);
    }
  }

  /**
   * Diagnostics-only: mirror a received payload's rms/peak back to the shell
   * via the `audio_diag_rx` command so the engine-side `CHROMATICITY_AUDIO_DIAG`
   * capture log shows webview-received values next to producer-emitted ones.
   * The shell no-ops the command unless diagnostics are enabled; failures
   * (outside Tauri) are swallowed.
   */
  async #reportRx(rms: number, peak: number, received: number): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('audio_diag_rx', { rms, peak, received });
    } catch {
      // diagnostics only — ignore (no Tauri runtime / command unavailable).
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
    this.received = 0;
    this.error = null;
  }
}

export const audioSnapshotStore = new AudioSnapshotStore();
