// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.4 Lane PERF.D OSC health polling store tests. Validates the
// `osc_health_snapshot` Tauri command invocation, snapshot state
// propagation, polling-loop idempotency, and the error-capture fallback
// when running outside Tauri.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { OscHealthSnapshot } from '../ipc/bindings/OscHealthSnapshot';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_HEALTHY: OscHealthSnapshot = {
  state: 'Healthy',
  attempted_binds: 1,
  local_addr: '0.0.0.0:7000',
  compat_alias_emits: 0n,
};

const SAMPLE_RECOVERING: OscHealthSnapshot = {
  state: 'Recovering',
  attempted_binds: 3,
  local_addr: null,
  compat_alias_emits: 0n,
};

describe('oscHealthStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('refresh() invokes osc_health_snapshot and stores the snapshot', async () => {
    invokeMock.mockResolvedValue(SAMPLE_HEALTHY);
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    await oscHealthStore.refresh();
    expect(invokeMock).toHaveBeenCalledWith('osc_health_snapshot');
    expect(oscHealthStore.snapshot).toEqual(SAMPLE_HEALTHY);
    expect(oscHealthStore.error).toBeNull();
  });

  it('refresh() captures errors and preserves the prior snapshot', async () => {
    invokeMock
      .mockResolvedValueOnce(SAMPLE_HEALTHY)
      .mockRejectedValueOnce(new Error('not running in tauri'));
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    await oscHealthStore.refresh();
    expect(oscHealthStore.snapshot).toEqual(SAMPLE_HEALTHY);
    await oscHealthStore.refresh();
    expect(oscHealthStore.error).toBe('not running in tauri');
    // Prior snapshot is retained — stale-but-last-good posture.
    expect(oscHealthStore.snapshot).toEqual(SAMPLE_HEALTHY);
  });

  it('refresh() coerces non-Error rejections to a string', async () => {
    invokeMock.mockRejectedValue('plain-string-error');
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    await oscHealthStore.refresh();
    expect(oscHealthStore.error).toBe('plain-string-error');
  });

  it('start() fires an immediate refresh and schedules a recurring poll', async () => {
    invokeMock.mockResolvedValue(SAMPLE_HEALTHY);
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    oscHealthStore.start(1000);
    // The immediate refresh is async — flush microtasks before the
    // first assertion.
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));
    // Advance the timer once to trigger the recurring poll.
    invokeMock.mockResolvedValue(SAMPLE_RECOVERING);
    await vi.advanceTimersByTimeAsync(1000);
    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(oscHealthStore.snapshot).toEqual(SAMPLE_RECOVERING);
    oscHealthStore.stop();
  });

  it('start() is idempotent — a second call does not schedule a duplicate timer', async () => {
    invokeMock.mockResolvedValue(SAMPLE_HEALTHY);
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    oscHealthStore.start(1000);
    oscHealthStore.start(1000); // second call — should be a no-op.
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));
    await vi.advanceTimersByTimeAsync(1000);
    // Exactly one additional invocation per tick (not two).
    expect(invokeMock).toHaveBeenCalledTimes(2);
    oscHealthStore.stop();
  });

  it('stop() halts the polling loop', async () => {
    invokeMock.mockResolvedValue(SAMPLE_HEALTHY);
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    oscHealthStore.start(1000);
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));
    oscHealthStore.stop();
    await vi.advanceTimersByTimeAsync(5000);
    // Only the eager first call should have fired.
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it('reset() clears snapshot, error, and stops polling', async () => {
    // Hold the eager refresh response open so we can assert reset()
    // semantics deterministically — without an in-flight invoke
    // resolving mid-test the post-reset snapshot stays cleared.
    let resolveFirst: (snapshot: OscHealthSnapshot) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<OscHealthSnapshot>((resolve) => {
          resolveFirst = resolve;
        }),
    );
    const { oscHealthStore } = await import('./oscHealth.svelte');
    oscHealthStore.reset();
    oscHealthStore.snapshot = SAMPLE_HEALTHY;
    oscHealthStore.error = 'stale error';
    oscHealthStore.start(1000);
    // Wait for the start() eager refresh() to dispatch its invoke()
    // call (deferred through the dynamic `import('@tauri-apps/api/core')`
    // resolution) — otherwise the post-reset call-count sample fires
    // before the eager refresh has reached the mock.
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));
    // Trigger reset BEFORE the held invoke resolves so the timer is
    // halted while the pending promise still holds; reset() halts
    // setInterval but cannot cancel in-flight invoke promises.
    oscHealthStore.reset();
    expect(oscHealthStore.snapshot).toBeNull();
    expect(oscHealthStore.error).toBeNull();
    const calls = invokeMock.mock.calls.length;
    // Advancing the (cleared) timer should not poll again.
    await vi.advanceTimersByTimeAsync(5000);
    expect(invokeMock.mock.calls.length).toBe(calls);
    // Release the held promise so the test does not leak a pending
    // microtask into the next test's fake-timer scope.
    resolveFirst(SAMPLE_HEALTHY);
    await Promise.resolve();
  });
});
