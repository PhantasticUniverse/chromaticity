// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Typed Tauri IPC wrappers for the registered MIDI commands surfaced by
// `app/shell/src/commands/midi.rs`. Keeping the command names here
// gives stores/views a typed boundary instead of scattering raw
// `invoke()` literals across the UI.

import { invoke } from '@tauri-apps/api/core';

import type { MidiDevicePayload } from './bindings/MidiDevicePayload';

export const MIDI_LEARN_EVENT = 'midi_learn_event';

const LIST_MIDI_DEVICES_COMMAND = 'list_midi_devices';
const START_MIDI_LEARN_COMMAND = 'start_midi_learn';
const STOP_MIDI_LEARN_COMMAND = 'stop_midi_learn';

export async function listMidiDevices(): Promise<MidiDevicePayload[]> {
  return invoke<MidiDevicePayload[]>(LIST_MIDI_DEVICES_COMMAND);
}

export async function startMidiLearn(portId: string): Promise<void> {
  await invoke(START_MIDI_LEARN_COMMAND, { portId });
}

export async function stopMidiLearn(): Promise<void> {
  await invoke(STOP_MIDI_LEARN_COMMAND);
}
