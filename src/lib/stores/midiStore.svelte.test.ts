// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane D.1 MIDI device-listing store tests. Validates the
// `list_midi_devices` Tauri command invocation, devices state
// propagation, loading-flag lifecycle, and the empty-vec graceful-
// fallback posture documented in the module-level prose.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { MidiDevicePayload } from '../ipc/bindings/MidiDevicePayload';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_DEVICE_A: MidiDevicePayload = {
  id: 'input-0',
  name: 'Launchpad X',
  kind: 'Input',
};

const SAMPLE_DEVICE_B: MidiDevicePayload = {
  id: 'input-1',
  name: 'APC40 mkII',
  kind: 'Input',
};

describe('midiStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('refresh() invokes list_midi_devices and stores the returned vec', async () => {
    invokeMock.mockResolvedValue([SAMPLE_DEVICE_A, SAMPLE_DEVICE_B]);
    const { midiStore } = await import('./midiStore.svelte');
    midiStore.reset();
    await midiStore.refresh();
    expect(invokeMock).toHaveBeenCalledWith('list_midi_devices');
    expect(midiStore.devices).toEqual([SAMPLE_DEVICE_A, SAMPLE_DEVICE_B]);
    expect(midiStore.error).toBeNull();
    expect(midiStore.isLoading).toBe(false);
  });

  it('refresh() surfaces an empty vec verbatim under the no-devices branch', async () => {
    // The adapter collapses the no-MIDI-subsystem error into an empty
    // vec; the store must surface that without erroring so the view's
    // "no MIDI devices detected" empty-state branch fires.
    invokeMock.mockResolvedValue([]);
    const { midiStore } = await import('./midiStore.svelte');
    midiStore.reset();
    await midiStore.refresh();
    expect(midiStore.devices).toEqual([]);
    expect(midiStore.error).toBeNull();
  });

  it('refresh() captures errors and clears the device list on failure', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { midiStore } = await import('./midiStore.svelte');
    midiStore.reset();
    midiStore.devices = [SAMPLE_DEVICE_A];
    await midiStore.refresh();
    expect(midiStore.devices).toEqual([]);
    expect(midiStore.error).toBe('not running in tauri');
    expect(midiStore.isLoading).toBe(false);
  });

  it('refresh() coerces non-Error rejections to a string', async () => {
    invokeMock.mockRejectedValue('plain-string-error');
    const { midiStore } = await import('./midiStore.svelte');
    midiStore.reset();
    await midiStore.refresh();
    expect(midiStore.error).toBe('plain-string-error');
  });

  it('isLoading toggles around the in-flight refresh promise', async () => {
    let resolveInvoke: (value: MidiDevicePayload[]) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<MidiDevicePayload[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { midiStore } = await import('./midiStore.svelte');
    midiStore.reset();
    const refreshPromise = midiStore.refresh();
    expect(midiStore.isLoading).toBe(true);
    resolveInvoke([SAMPLE_DEVICE_A]);
    await refreshPromise;
    expect(midiStore.isLoading).toBe(false);
    expect(midiStore.devices).toEqual([SAMPLE_DEVICE_A]);
  });

  it('reset() clears devices, loading, and error state', async () => {
    invokeMock.mockResolvedValue([SAMPLE_DEVICE_A]);
    const { midiStore } = await import('./midiStore.svelte');
    await midiStore.refresh();
    midiStore.error = 'old error';
    midiStore.reset();
    expect(midiStore.devices).toEqual([]);
    expect(midiStore.error).toBeNull();
    expect(midiStore.isLoading).toBe(false);
  });
});
