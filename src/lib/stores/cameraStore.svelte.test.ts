// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 1c-β Phase 4 Lane A camera-store tests. Validates the
// `list_camera_devices` Tauri command invocation, devices state
// propagation, selection + localStorage persistence round-trip,
// loading-flag lifecycle, and the empty-vec stub posture documented in
// the module-level prose.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { CameraDevicePayload } from '../ipc/bindings/CameraDevicePayload';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_DEVICE_A: CameraDevicePayload = {
  device_id: 'cam-built-in-0',
  display_name: 'FaceTime HD',
  kind: 'BuiltIn',
};

const SAMPLE_DEVICE_B: CameraDevicePayload = {
  device_id: 'cam-external-1',
  display_name: 'Logitech Brio',
  kind: 'External',
};

describe('cameraStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it('refresh() invokes list_camera_devices and stores the returned vec', async () => {
    invokeMock.mockResolvedValue([SAMPLE_DEVICE_A, SAMPLE_DEVICE_B]);
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    await cameraStore.refresh();
    expect(invokeMock).toHaveBeenCalledWith('list_camera_devices');
    expect(cameraStore.devices).toEqual([SAMPLE_DEVICE_A, SAMPLE_DEVICE_B]);
    expect(cameraStore.error).toBeNull();
    expect(cameraStore.isLoading).toBe(false);
  });

  it('refresh() surfaces an empty vec verbatim under the scaffold stub', async () => {
    // Phase 4 Lane A documented stub: the per-platform nokhwa
    // enumeration arms are not wired yet, so the command returns an
    // empty vec. The store must surface that without erroring so the
    // view's "no cameras found" empty-state branch fires.
    invokeMock.mockResolvedValue([]);
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    await cameraStore.refresh();
    expect(cameraStore.devices).toEqual([]);
    expect(cameraStore.error).toBeNull();
  });

  it('refresh() captures errors and clears the device list on failure', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    cameraStore.devices = [SAMPLE_DEVICE_A];
    await cameraStore.refresh();
    expect(cameraStore.devices).toEqual([]);
    expect(cameraStore.error).toBe('not running in tauri');
    expect(cameraStore.isLoading).toBe(false);
  });

  it('select(id) updates `selected` and persists to localStorage', async () => {
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    cameraStore.select('cam-built-in-0');
    expect(cameraStore.selected).toBe('cam-built-in-0');
    expect(localStorage.getItem('chromaticity.camera.selectedDevice')).toBe('cam-built-in-0');
  });

  it('select(null) clears `selected` and removes the persisted key', async () => {
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    cameraStore.select('cam-external-1');
    cameraStore.select(null);
    expect(cameraStore.selected).toBeNull();
    expect(localStorage.getItem('chromaticity.camera.selectedDevice')).toBeNull();
  });

  it('loadPersistedSelection() reads the prior selection on init', async () => {
    localStorage.setItem('chromaticity.camera.selectedDevice', 'cam-external-1');
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    cameraStore.loadPersistedSelection();
    expect(cameraStore.selected).toBe('cam-external-1');
  });

  it('loadPersistedSelection() leaves `selected` at null when nothing is persisted', async () => {
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    cameraStore.loadPersistedSelection();
    expect(cameraStore.selected).toBeNull();
  });

  it('isLoading toggles around the in-flight refresh promise', async () => {
    let resolveInvoke: (value: CameraDevicePayload[]) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<CameraDevicePayload[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { cameraStore } = await import('./cameraStore.svelte');
    cameraStore.reset();
    const refreshPromise = cameraStore.refresh();
    expect(cameraStore.isLoading).toBe(true);
    resolveInvoke([SAMPLE_DEVICE_A]);
    await refreshPromise;
    expect(cameraStore.isLoading).toBe(false);
    expect(cameraStore.devices).toEqual([SAMPLE_DEVICE_A]);
  });
});
