// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Persistent MIDI-learn -> live control-target binding & performance listener store.
 *
 * Maps learned `MidiLearnEventPayload` signatures (device + channel + kind +
 * number) to concrete scene/meta control targets (`controlId`), persisted in
 * `localStorage`, and subscribes to the Rust `midi_performance_event` stream
 * (`start_midi_performance_listener`) so incoming MIDI CC/Note events drive
 * controls live during performance.
 */

import { dispatchAndRead } from './control';
import type { MidiLearnEventPayload } from '../ipc/bindings/MidiLearnEventPayload';
import { registerStateMachine } from '../state';

export const MIDI_BINDINGS_STORAGE_KEY = 'chromaticity.midi.bindings.v1';
export const MIDI_PERFORMANCE_EVENT = 'midi_performance_event';

export interface MidiControlBinding {
  readonly id: string;
  readonly target: string;
  readonly event: MidiLearnEventPayload;
  readonly createdAtMs: number;
}

function bindingKeyFor(event: MidiLearnEventPayload): string {
  return `${event.port_id}:${event.channel ?? 'any'}:${event.kind}:${event.note_or_cc ?? 'any'}`;
}

/**
 * Extract a numeric `controlId` from a binding target string
 * (e.g. `"control:3"`, `"meta:1002"`, `"scene.4"`, or `"7"`).
 */
export function parseMidiTargetControlId(target: string): number | null {
  const trimmed = target.trim();
  const match = /^(?:(?:control|meta|scene)[:.])?(\d+)$/i.exec(trimmed);
  if (!match || !match[1]) return null;
  const parsed = Number.parseInt(match[1], 10);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
}

function loadPersistedBindings(): MidiControlBinding[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(MIDI_BINDINGS_STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as MidiControlBinding[]) : [];
  } catch {
    return [];
  }
}

class MidiBindingsStore {
  bindings = $state<readonly MidiControlBinding[]>(loadPersistedBindings());
  listening = $state<boolean>(false);
  activePorts = $state<number>(0);

  #unlisten: (() => void) | null = null;

  /** Bind a learned MIDI event to `target`, replacing any prior binding on that target or event key. */
  bind(target: string, event: MidiLearnEventPayload): MidiControlBinding {
    const id = bindingKeyFor(event);
    const entry: MidiControlBinding = {
      id,
      target,
      event,
      createdAtMs: Date.now(),
    };
    this.bindings = [...this.bindings.filter((b) => b.id !== id && b.target !== target), entry];
    this.#persist();
    if (!this.listening) {
      void this.startListening();
    }
    return entry;
  }

  /** Remove a binding by its event key id. */
  unbind(id: string): void {
    this.bindings = this.bindings.filter((b) => b.id !== id);
    this.#persist();
  }

  /** Look up the target bound to an incoming MIDI event, or `null`. */
  targetFor(event: MidiLearnEventPayload): string | null {
    const id = bindingKeyFor(event);
    return this.bindings.find((b) => b.id === id)?.target ?? null;
  }

  /**
   * Dispatch a live incoming MIDI event to its bound control target.
   * Returns `true` iff a matching binding with a numeric control ID was dispatched.
   */
  dispatchIncomingEvent(event: MidiLearnEventPayload): boolean {
    const target = this.targetFor(event);
    if (target === null) return false;
    const controlId = parseMidiTargetControlId(target);
    if (controlId === null) return false;

    const rawVal = event.velocity_or_value ?? 0;
    const normalized = Math.max(0, Math.min(1, rawVal / 127));
    void dispatchAndRead(controlId, normalized).catch(() => {});
    return true;
  }

  /**
   * Start persistent live MIDI performance listening across all connected
   * host MIDI input ports via `start_midi_performance_listener`.
   */
  async startListening(): Promise<number> {
    if (this.listening) return this.activePorts;
    try {
      const [{ invoke }, { listen }] = await Promise.all([
        import('@tauri-apps/api/core'),
        import('@tauri-apps/api/event'),
      ]);
      const unlisten = await listen<MidiLearnEventPayload>(MIDI_PERFORMANCE_EVENT, (ev) => {
        this.dispatchIncomingEvent(ev.payload);
      });
      const count = await invoke<number>('start_midi_performance_listener');
      this.#unlisten = unlisten;
      this.activePorts = count;
      this.listening = true;
      return count;
    } catch {
      // Outside Tauri (browser preview / Vitest): stay cleanly idle.
      return 0;
    }
  }

  /** Stop the persistent background MIDI performance listener. */
  async stopListening(): Promise<void> {
    if (this.#unlisten !== null) {
      this.#unlisten();
      this.#unlisten = null;
    }
    this.listening = false;
    this.activePorts = 0;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('stop_midi_performance_listener');
    } catch {
      // Outside Tauri: no-op.
    }
  }

  reset(): void {
    if (this.#unlisten !== null) {
      this.#unlisten();
      this.#unlisten = null;
    }
    this.listening = false;
    this.activePorts = 0;
    this.bindings = [];
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.removeItem(MIDI_BINDINGS_STORAGE_KEY);
      } catch {
        // Ignore storage errors in restricted environments.
      }
    }
  }

  #persist(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(MIDI_BINDINGS_STORAGE_KEY, JSON.stringify(this.bindings));
    } catch {
      // Ignore quota / private-mode errors.
    }
  }
}

export const midiBindingsStore = new MidiBindingsStore();

registerStateMachine('midiBindings', () => ({
  count: midiBindingsStore.bindings.length,
  listening: midiBindingsStore.listening,
  activePorts: midiBindingsStore.activePorts,
}));
