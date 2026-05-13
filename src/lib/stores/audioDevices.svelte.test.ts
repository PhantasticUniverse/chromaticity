// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.6 Lane AUD.A cpal input-device-listing store tests. Validates
// the `list_audio_devices` Tauri command invocation, devices state
// propagation, loading-flag lifecycle, and the empty-vec graceful-
// fallback posture documented in the module-level prose.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { AudioDevicePayload } from '../ipc/bindings/AudioDevicePayload';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_DEVICE_A: AudioDevicePayload = {
  id: 'input-built-in-microphone',
  name: 'Built-in Microphone',
  kind: 'Input',
};

const SAMPLE_DEVICE_B: AudioDevicePayload = {
  id: 'input-blackhole-16ch',
  name: 'BlackHole 16ch',
  kind: 'Input',
};

describe('audioDevicesStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('refresh() invokes list_audio_devices and stores the returned vec', async () => {
    invokeMock.mockResolvedValue([SAMPLE_DEVICE_A, SAMPLE_DEVICE_B]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    await audioDevicesStore.refresh();
    expect(invokeMock).toHaveBeenCalledWith('list_audio_devices');
    expect(audioDevicesStore.devices).toEqual([SAMPLE_DEVICE_A, SAMPLE_DEVICE_B]);
    expect(audioDevicesStore.error).toBeNull();
    expect(audioDevicesStore.isLoading).toBe(false);
  });

  it('refresh() surfaces an empty vec verbatim under the no-devices branch', async () => {
    // The engine collapses the no-audio-subsystem error into an empty
    // vec; the store must surface that without erroring so the
    // Settings view's fixture-fallback branch fires.
    invokeMock.mockResolvedValue([]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    await audioDevicesStore.refresh();
    expect(audioDevicesStore.devices).toEqual([]);
    expect(audioDevicesStore.error).toBeNull();
  });

  it('refresh() captures errors and clears the device list on failure', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    audioDevicesStore.devices = [SAMPLE_DEVICE_A];
    await audioDevicesStore.refresh();
    expect(audioDevicesStore.devices).toEqual([]);
    expect(audioDevicesStore.error).toBe('not running in tauri');
    expect(audioDevicesStore.isLoading).toBe(false);
  });

  it('refresh() coerces non-Error rejections to a string', async () => {
    invokeMock.mockRejectedValue('plain-string-error');
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    await audioDevicesStore.refresh();
    expect(audioDevicesStore.error).toBe('plain-string-error');
  });

  it('isLoading toggles around the in-flight refresh promise', async () => {
    let resolveInvoke: (value: AudioDevicePayload[]) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<AudioDevicePayload[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    const refreshPromise = audioDevicesStore.refresh();
    expect(audioDevicesStore.isLoading).toBe(true);
    resolveInvoke([SAMPLE_DEVICE_A]);
    await refreshPromise;
    expect(audioDevicesStore.isLoading).toBe(false);
    expect(audioDevicesStore.devices).toEqual([SAMPLE_DEVICE_A]);
  });

  it('reset() clears devices, loading, and error state', async () => {
    invokeMock.mockResolvedValue([SAMPLE_DEVICE_A]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    await audioDevicesStore.refresh();
    audioDevicesStore.error = 'old error';
    audioDevicesStore.reset();
    expect(audioDevicesStore.devices).toEqual([]);
    expect(audioDevicesStore.error).toBeNull();
    expect(audioDevicesStore.isLoading).toBe(false);
  });
});
