// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.2 MIDI learn-mode singleton store. Backs the
// operator-facing MIDI view learn-mode panel — `start(portId)` invokes
// the `start_midi_learn` Tauri command (`app/shell/src/commands/midi.rs`),
// subscribes to the `midi_learn_event` event channel, and records each
// captured payload onto `captured`. `stop()` invokes `stop_midi_learn`
// and detaches the listener.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// `audioSnapshot.svelte.ts` sibling is the live precedent for the
// listener-shape pattern (idempotent `subscribe()` + dynamic-import +
// no-Tauri-runtime swallow path).
//
// **State machine** (Wave 2.1 Lane D.2 minimal-viable single capture):
//
//   idle  ── start(portId)  ──▶  listening
//   listening ── first event ──▶  captured   (replaces `captured`)
//   captured ── stop() ──▶ idle
//   listening ── stop() ──▶ idle
//   any-state ── reset() ──▶ idle (also clears `captured`)
//
// The store stays in `'listening'` after the first event so subsequent
// captures replace the displayed payload (re-capture without an extra
// click). The UI surfaces a `state === 'captured'` view by re-checking
// `captured !== null` against the `listening` state. Multi-message
// learn (combining several CC ranges) is deferred to a follow-up wave
// per the brief.
//
// **localStorage persistence**: Wave 2.1 Lane D.2 stores the most
// recently captured payload to a fixed key on `confirm()`. The wiring
// of this captured binding to a specific control target (the per-row
// Bind button in the MIDI view's binding tables) is a follow-up wave;
// today, `confirm()` just records the most-recent capture so the
// operator can verify the learn-mode loop works end-to-end and a
// future wave can read this localStorage entry as the seed for the
// binding-assignment UX.

import type { MidiLearnEventPayload } from '../ipc/bindings/MidiLearnEventPayload';
import { MIDI_LEARN_EVENT, startMidiLearn, stopMidiLearn } from '../ipc/midi';

/** Tauri unlisten handle returned by `listen()`. Stored so `stop()` can detach. */
type UnlistenFn = () => void;

/** localStorage key under which the most recent confirmed capture is recorded. */
export const MIDI_LEARN_CONFIRMED_KEY = 'chromaticity.midi.learn.confirmed';

/** Discriminated state for the learn-mode singleton store. */
export type MidiLearnState = 'idle' | 'listening' | 'captured';

class MidiLearnStore {
  /** Current state-machine state. */
  state = $state<MidiLearnState>('idle');

  /** Most recent captured payload, or `null` if nothing captured yet. */
  captured = $state<MidiLearnEventPayload | null>(null);

  /** Port id the active session is listening on (or `null` when idle). */
  activePortId = $state<string | null>(null);

  /** Last operator-facing error from a failed `start_midi_learn` invocation. */
  error = $state<string | null>(null);

  /** Detach handle returned by `listen()`. */
  #unlisten: UnlistenFn | null = null;

  /**
   * Start a learn session on `portId`. Invokes `start_midi_learn` on
   * the shell, transitions state to `'listening'`, and subscribes to
   * the `midi_learn_event` event channel. Replaces any prior session
   * (stop + restart is the intentional UX for switching ports).
   */
  async start(portId: string): Promise<void> {
    // Always tear down any prior listener before opening a fresh one —
    // matches the shell-side replace-on-install posture for the
    // `MidiListenHandle` slot on `AppState`.
    await this.#detachListener();

    this.error = null;
    this.captured = null;
    this.activePortId = portId;
    this.state = 'listening';

    try {
      await startMidiLearn(portId);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.state = 'idle';
      this.activePortId = null;
      return;
    }

    try {
      const { listen } = await import('@tauri-apps/api/event');
      const handle = await listen<MidiLearnEventPayload>(MIDI_LEARN_EVENT, (event) => {
        this.captured = event.payload;
        // Transition to `'captured'` on the first event; subsequent
        // events keep replacing `captured` while the operator stays
        // in this state (re-capture without an explicit re-arm).
        this.state = 'captured';
      });
      this.#unlisten = handle;
    } catch (e) {
      // Listener attach failed (no Tauri runtime — vite-dev or Vitest
      // jsdom without IPC mock). The shell side has installed its
      // listen handle; absent the frontend listener, no events surface,
      // but state-machine semantics stay consistent. Mirrors the
      // `audioSnapshot.svelte.ts` no-Tauri-runtime swallow path.
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /**
   * Stop the active learn session. Invokes `stop_midi_learn` on the
   * shell, detaches the event listener, and returns to `'idle'`.
   * Captured payload is preserved (the operator can still see what
   * was last captured); `reset()` clears it.
   */
  async stop(): Promise<void> {
    await this.#detachListener();
    try {
      await stopMidiLearn();
    } catch (e) {
      // Swallow — outside Tauri the command rejects; nothing to clean
      // up server-side and the state-machine transition still applies.
      this.error = e instanceof Error ? e.message : String(e);
    }
    this.state = 'idle';
    this.activePortId = null;
  }

  /**
   * Confirm the most-recently captured payload by writing it to
   * localStorage under [`MIDI_LEARN_CONFIRMED_KEY`]. Wave 2.1 Lane D.2
   * minimal-viable surface: the captured binding is recorded as a
   * "most recent confirmed" record so a future wave can use it as
   * the seed for the binding-assignment UX. Does NOT detach the
   * listener — the operator may want to re-capture without leaving
   * learn mode. Returns the confirmed payload (or `null` if nothing
   * captured yet).
   */
  confirm(): MidiLearnEventPayload | null {
    if (!this.captured) return null;
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(MIDI_LEARN_CONFIRMED_KEY, JSON.stringify(this.captured));
      }
    } catch {
      // localStorage may be unavailable (private browsing quota
      // exhaustion, etc.); swallow rather than fail the confirm UX —
      // the in-memory `captured` field still carries the value.
    }
    return this.captured;
  }

  /** Clear the captured payload + state without invoking IPC. Used by
   *  the Re-capture button to drop the previous capture and stay in
   *  `'listening'` mode. */
  reset(): void {
    this.captured = null;
    if (this.state === 'captured') {
      this.state = 'listening';
    }
  }

  async #detachListener(): Promise<void> {
    if (this.#unlisten) {
      try {
        this.#unlisten();
      } catch {
        // Tauri's unlisten may throw if the runtime is shutting down;
        // swallow so the state-machine transition completes.
      }
      this.#unlisten = null;
    }
  }
}

export const midiLearnStore = new MidiLearnStore();
