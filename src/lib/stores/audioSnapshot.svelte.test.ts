// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 6 PR 2 audio-uniform IPC consumer store tests. Validates the
// `audio_uniform_snapshot_changed` Tauri event subscription, snapshot
// state propagation, idempotency of repeated `subscribe()` calls, the
// `unsubscribe()` detach handle, and the no-Tauri-runtime swallow path.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';

const { listenMock, unlistenMock } = vi.hoisted(() => ({
  listenMock: vi.fn(),
  unlistenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

const SAMPLE_PAYLOAD: AudioUniformSnapshotPayload = {
  bpm: 128,
  rms: 0.5,
  peak: 0.8,
  fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
};

describe('audioSnapshotStore', () => {
  beforeEach(() => {
    listenMock.mockReset();
    unlistenMock.mockReset();
    listenMock.mockResolvedValue(unlistenMock);
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('subscribes to audio_uniform_snapshot_changed with the correct event name', async () => {
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    await audioSnapshotStore.subscribe();
    expect(listenMock).toHaveBeenCalledTimes(1);
    expect(listenMock).toHaveBeenCalledWith('audio_uniform_snapshot_changed', expect.any(Function));
  });

  it('stores received payloads on `snapshot`', async () => {
    let captured: ((event: { payload: AudioUniformSnapshotPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: AudioUniformSnapshotPayload }) => void;
      return unlistenMock;
    });
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    await audioSnapshotStore.subscribe();
    expect(captured).not.toBeNull();
    captured!({ payload: SAMPLE_PAYLOAD });
    expect(audioSnapshotStore.snapshot).toEqual(SAMPLE_PAYLOAD);
    audioSnapshotStore.unsubscribe();
  });

  it('only registers one listener even when subscribe() is called repeatedly (idempotency)', async () => {
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    await audioSnapshotStore.subscribe();
    await audioSnapshotStore.subscribe();
    await audioSnapshotStore.subscribe();
    expect(listenMock).toHaveBeenCalledTimes(1);
    audioSnapshotStore.unsubscribe();
  });

  it('returns the same promise for concurrent subscribe() calls', async () => {
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    const p1 = audioSnapshotStore.subscribe();
    const p2 = audioSnapshotStore.subscribe();
    expect(p1).toBe(p2);
    await Promise.all([p1, p2]);
    audioSnapshotStore.unsubscribe();
  });

  it('unsubscribe() calls the unlisten handle and clears the snapshot', async () => {
    let captured: ((event: { payload: AudioUniformSnapshotPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: AudioUniformSnapshotPayload }) => void;
      return unlistenMock;
    });
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    await audioSnapshotStore.subscribe();
    captured!({ payload: SAMPLE_PAYLOAD });
    expect(audioSnapshotStore.snapshot).not.toBeNull();
    audioSnapshotStore.unsubscribe();
    expect(unlistenMock).toHaveBeenCalledTimes(1);
    expect(audioSnapshotStore.snapshot).toBeNull();
  });

  it('allows resubscription after unsubscribe()', async () => {
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    await audioSnapshotStore.subscribe();
    audioSnapshotStore.unsubscribe();
    await audioSnapshotStore.subscribe();
    expect(listenMock).toHaveBeenCalledTimes(2);
    audioSnapshotStore.unsubscribe();
  });

  it('swallows listen() rejection (no Tauri runtime) and leaves snapshot at null', async () => {
    listenMock.mockRejectedValue(new Error('not running in tauri'));
    const { audioSnapshotStore } = await import('./audioSnapshot.svelte');
    await expect(audioSnapshotStore.subscribe()).resolves.toBeUndefined();
    expect(audioSnapshotStore.snapshot).toBeNull();
  });
});
