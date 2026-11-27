// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Persistent MIDI-learn → control-target binding store (closes G6 item #2).
 *
 * Maps learned `MidiLearnEventPayload` signatures (device + channel + kind +
 * number) to concrete scene/meta control targets (`controlId`), persisted in
 * `localStorage` and registered in the read-only state-machine diagnostics.
 */

import type { MidiLearnEventPayload } from '../ipc/bindings/MidiLearnEventPayload';
import { registerStateMachine } from '../state';

export const MIDI_BINDINGS_STORAGE_KEY = 'chromaticity.midi.bindings.v1';

export interface MidiControlBinding {
  readonly id: string;
  readonly target: string;
  readonly event: MidiLearnEventPayload;
  readonly createdAtMs: number;
}

function bindingKeyFor(event: MidiLearnEventPayload): string {
  return `${event.port_id}:${event.channel ?? 'any'}:${event.kind}:${event.note_or_cc ?? 'any'}`;
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

  reset(): void {
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
}));
