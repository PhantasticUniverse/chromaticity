// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
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
  max_channels: 2,
};

const SAMPLE_DEVICE_B: AudioDevicePayload = {
  id: 'input-blackhole-16ch',
  name: 'BlackHole 16ch',
  kind: 'Input',
  max_channels: 2,
};

const SAMPLE_OUTPUT_A: AudioDevicePayload = {
  id: 'output-built-in-output',
  name: 'Built-in Output',
  kind: 'Output',
  max_channels: 2,
};

const SAMPLE_OUTPUT_B: AudioDevicePayload = {
  id: 'output-external-interface',
  name: 'External Audio Interface',
  kind: 'Output',
  max_channels: 2,
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

  // ---- Wave 2.7 Lane SET.C output-enumeration cases ----

  it('refreshOutput() invokes list_audio_output_devices and stores the returned vec', async () => {
    invokeMock.mockResolvedValue([SAMPLE_OUTPUT_A, SAMPLE_OUTPUT_B]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    await audioDevicesStore.refreshOutput();
    expect(invokeMock).toHaveBeenCalledWith('list_audio_output_devices');
    expect(audioDevicesStore.outputDevices).toEqual([SAMPLE_OUTPUT_A, SAMPLE_OUTPUT_B]);
    expect(audioDevicesStore.outputError).toBeNull();
    expect(audioDevicesStore.isLoadingOutput).toBe(false);
  });

  it('refreshOutput() surfaces an empty vec verbatim under the no-devices branch', async () => {
    invokeMock.mockResolvedValue([]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    await audioDevicesStore.refreshOutput();
    expect(audioDevicesStore.outputDevices).toEqual([]);
    expect(audioDevicesStore.outputError).toBeNull();
  });

  it('refreshOutput() captures errors and clears the output device list on failure', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    audioDevicesStore.outputDevices = [SAMPLE_OUTPUT_A];
    await audioDevicesStore.refreshOutput();
    expect(audioDevicesStore.outputDevices).toEqual([]);
    expect(audioDevicesStore.outputError).toBe('not running in tauri');
    expect(audioDevicesStore.isLoadingOutput).toBe(false);
  });

  it('refreshOutput() coerces non-Error rejections to a string', async () => {
    invokeMock.mockRejectedValue('plain-string-error');
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    await audioDevicesStore.refreshOutput();
    expect(audioDevicesStore.outputError).toBe('plain-string-error');
  });

  it('isLoadingOutput toggles around the in-flight refreshOutput promise', async () => {
    let resolveInvoke: (value: AudioDevicePayload[]) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<AudioDevicePayload[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    const refreshPromise = audioDevicesStore.refreshOutput();
    expect(audioDevicesStore.isLoadingOutput).toBe(true);
    resolveInvoke([SAMPLE_OUTPUT_A]);
    await refreshPromise;
    expect(audioDevicesStore.isLoadingOutput).toBe(false);
    expect(audioDevicesStore.outputDevices).toEqual([SAMPLE_OUTPUT_A]);
  });

  it('reset() clears output state alongside input state', async () => {
    invokeMock.mockResolvedValue([SAMPLE_OUTPUT_A]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    await audioDevicesStore.refreshOutput();
    audioDevicesStore.outputError = 'stale output error';
    audioDevicesStore.reset();
    expect(audioDevicesStore.outputDevices).toEqual([]);
    expect(audioDevicesStore.outputError).toBeNull();
    expect(audioDevicesStore.isLoadingOutput).toBe(false);
  });

  it('refresh() and refreshOutput() are independent — input refresh does not clear outputs', async () => {
    // Seed the output side manually (no refreshOutput call), then run
    // a vanilla input refresh: the output list MUST survive. Pins the
    // independence of the two enumeration tracks — a regression that
    // shared state between the two refresh paths would surface here.
    invokeMock.mockResolvedValue([SAMPLE_DEVICE_A]);
    const { audioDevicesStore } = await import('./audioDevices.svelte');
    audioDevicesStore.reset();
    audioDevicesStore.outputDevices = [SAMPLE_OUTPUT_A];
    await audioDevicesStore.refresh();
    expect(audioDevicesStore.devices).toEqual([SAMPLE_DEVICE_A]);
    expect(audioDevicesStore.outputDevices).toEqual([SAMPLE_OUTPUT_A]);
  });
});
