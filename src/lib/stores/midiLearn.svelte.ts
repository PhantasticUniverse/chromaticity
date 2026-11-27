// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.2 MIDI learn-mode singleton store, upgraded to an
// explicit discriminated union (`MidiLearnMachine`) where invalid
// combinations (e.g. listening without a portId or captured without a
// payload) are unrepresentable in TypeScript.

import type { MidiLearnEventPayload } from '../ipc/bindings/MidiLearnEventPayload';
import { MIDI_LEARN_EVENT, startMidiLearn, stopMidiLearn } from '../ipc/midi';
import { formatErrorMessage, registerStateMachine } from '../state';

type UnlistenFn = () => void;

/** localStorage key under which the most recent confirmed capture is recorded. */
export const MIDI_LEARN_CONFIRMED_KEY = 'chromaticity.midi.learn.confirmed';

/** Back-compat state label for existing callers and tests. */
export type MidiLearnState = 'idle' | 'listening' | 'captured';

/** Explicit discriminated state machine for MIDI learn sessions. */
export type MidiLearnMachine =
  | {
      readonly kind: 'idle';
      readonly lastCaptured: MidiLearnEventPayload | null;
      readonly error: string | null;
    }
  | { readonly kind: 'starting'; readonly portId: string }
  | { readonly kind: 'listening'; readonly portId: string; readonly error: string | null }
  | {
      readonly kind: 'captured';
      readonly portId: string;
      readonly payload: MidiLearnEventPayload;
    };

class MidiLearnStore {
  /** Explicit discriminated state machine. */
  machine = $state<MidiLearnMachine>({ kind: 'idle', lastCaptured: null, error: null });

  /** Monotonic session counter to ignore stale async start/listen completions. */
  #sessionId = 0;

  /** Detach handle returned by `listen()`. */
  #unlisten: UnlistenFn | null = null;

  /** Back-compat projection: `'idle' | 'listening' | 'captured'`. */
  get state(): MidiLearnState {
    return this.machine.kind === 'starting' ? 'listening' : this.machine.kind;
  }

  set state(next: MidiLearnState) {
    const portId = this.activePortId ?? '';
    const err = this.error;
    const cap = this.captured;
    if (next === 'idle') {
      this.machine = { kind: 'idle', lastCaptured: cap, error: err };
    } else if (next === 'listening') {
      this.machine = { kind: 'listening', portId, error: err };
    } else if (cap !== null) {
      this.machine = { kind: 'captured', portId, payload: cap };
    } else {
      this.machine = { kind: 'listening', portId, error: err };
    }
  }

  /** Back-compat projection: most recent captured payload, or `null`. */
  get captured(): MidiLearnEventPayload | null {
    switch (this.machine.kind) {
      case 'captured':
        return this.machine.payload;
      case 'idle':
        return this.machine.lastCaptured;
      case 'starting':
      case 'listening':
        return null;
    }
  }

  set captured(payload: MidiLearnEventPayload | null) {
    if (this.machine.kind === 'idle') {
      this.machine = { kind: 'idle', lastCaptured: payload, error: this.machine.error };
    } else if (payload !== null) {
      this.machine = { kind: 'captured', portId: this.machine.portId, payload };
    } else {
      this.machine = { kind: 'listening', portId: this.machine.portId, error: this.error };
    }
  }

  /** Back-compat projection: active port id, or `null` when idle. */
  get activePortId(): string | null {
    return this.machine.kind === 'idle' ? null : this.machine.portId;
  }

  set activePortId(portId: string | null) {
    if (portId === null) {
      this.machine = { kind: 'idle', lastCaptured: this.captured, error: this.error };
    } else if (this.machine.kind === 'captured') {
      this.machine = { kind: 'captured', portId, payload: this.machine.payload };
    } else {
      this.machine = { kind: 'listening', portId, error: this.error };
    }
  }

  /** Back-compat projection: last operator-facing error, or `null`. */
  get error(): string | null {
    switch (this.machine.kind) {
      case 'idle':
      case 'listening':
        return this.machine.error;
      case 'starting':
      case 'captured':
        return null;
    }
  }

  set error(value: string | null) {
    if (this.machine.kind === 'idle') {
      this.machine = { kind: 'idle', lastCaptured: this.machine.lastCaptured, error: value };
    } else if (this.machine.kind === 'listening') {
      this.machine = { kind: 'listening', portId: this.machine.portId, error: value };
    }
  }

  async start(portId: string): Promise<void> {
    const sid = ++this.#sessionId;
    await this.#detachListener();
    if (sid !== this.#sessionId) return;

    this.machine = { kind: 'starting', portId };

    try {
      await startMidiLearn(portId);
    } catch (e) {
      if (sid !== this.#sessionId) return;
      this.machine = { kind: 'idle', lastCaptured: null, error: formatErrorMessage(e) };
      return;
    }

    if (sid !== this.#sessionId) return;
    this.machine = { kind: 'listening', portId, error: null };

    try {
      const { listen } = await import('@tauri-apps/api/event');
      const handle = await listen<MidiLearnEventPayload>(MIDI_LEARN_EVENT, (event) => {
        if (sid !== this.#sessionId) return;
        this.machine = { kind: 'captured', portId, payload: event.payload };
      });
      if (sid !== this.#sessionId) {
        try {
          handle();
        } catch {
          // Swallow detach failure on superseded session.
        }
        return;
      }
      this.#unlisten = handle;
    } catch (e) {
      if (sid !== this.#sessionId) return;
      this.machine = { kind: 'listening', portId, error: formatErrorMessage(e) };
    }
  }

  async stop(): Promise<void> {
    this.#sessionId += 1;
    const preserved = this.captured;
    await this.#detachListener();
    let stopError: string | null = null;
    try {
      await stopMidiLearn();
    } catch (e) {
      stopError = formatErrorMessage(e);
    }
    this.machine = { kind: 'idle', lastCaptured: preserved, error: stopError };
  }

  confirm(): MidiLearnEventPayload | null {
    const payload = this.captured;
    if (!payload) return null;
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(MIDI_LEARN_CONFIRMED_KEY, JSON.stringify(payload));
      }
    } catch {
      // localStorage may be unavailable; in-memory capture stays intact.
    }
    return payload;
  }

  reset(): void {
    switch (this.machine.kind) {
      case 'captured':
        this.machine = { kind: 'listening', portId: this.machine.portId, error: null };
        break;
      case 'idle':
        this.machine = { kind: 'idle', lastCaptured: null, error: this.machine.error };
        break;
      case 'starting':
      case 'listening':
        break;
    }
  }

  async #detachListener(): Promise<void> {
    if (this.#unlisten) {
      try {
        this.#unlisten();
      } catch {
        // Tauri's unlisten may throw during shutdown; swallow cleanly.
      }
      this.#unlisten = null;
    }
  }
}

export const midiLearnStore = new MidiLearnStore();

registerStateMachine('midiLearn', () => ({
  kind: midiLearnStore.machine.kind,
  activePortId: midiLearnStore.activePortId,
  hasCapture: midiLearnStore.captured !== null,
  error: midiLearnStore.error,
}));
