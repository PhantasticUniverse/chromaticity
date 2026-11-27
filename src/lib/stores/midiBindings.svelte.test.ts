// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, test } from 'vitest';

import { MIDI_BINDINGS_STORAGE_KEY, midiBindingsStore } from './midiBindings.svelte';
import type { MidiLearnEventPayload } from '../ipc/bindings/MidiLearnEventPayload';

const SAMPLE_CC: MidiLearnEventPayload = {
  port_id: 'midi-0',
  raw_bytes: [0xb0, 74, 100],
  channel: 1,
  kind: 'ControlChange',
  note_or_cc: 74,
  velocity_or_value: 100,
};

describe('midiBindingsStore', () => {
  beforeEach(() => {
    midiBindingsStore.reset();
  });

  test('binds, looks up, persists, and unbinds learned MIDI events', () => {
    const entry = midiBindingsStore.bind('scene.warp_amount', SAMPLE_CC);
    expect(midiBindingsStore.bindings).toHaveLength(1);
    expect(midiBindingsStore.targetFor(SAMPLE_CC)).toBe('scene.warp_amount');
    expect(localStorage.getItem(MIDI_BINDINGS_STORAGE_KEY)).toContain('scene.warp_amount');

    // Rebinding the same CC replaces the prior target cleanly.
    midiBindingsStore.bind('scene.speed', SAMPLE_CC);
    expect(midiBindingsStore.bindings).toHaveLength(1);
    expect(midiBindingsStore.targetFor(SAMPLE_CC)).toBe('scene.speed');

    midiBindingsStore.unbind(entry.id);
    expect(midiBindingsStore.bindings).toHaveLength(0);
    expect(midiBindingsStore.targetFor(SAMPLE_CC)).toBeNull();
  });
});
