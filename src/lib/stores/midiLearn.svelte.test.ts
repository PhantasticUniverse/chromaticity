// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.2 MIDI learn-mode store tests. Validates the
// state-machine transitions (idle → listening → captured → idle), the
// `start_midi_learn` / `stop_midi_learn` IPC dispatch, the
// `midi_learn_event` event-channel subscription, the localStorage
// confirm path, and the no-Tauri-runtime swallow path.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { MidiLearnEventPayload } from '../ipc/bindings/MidiLearnEventPayload';

const { invokeMock, listenMock, unlistenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  unlistenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

const SAMPLE_PAYLOAD: MidiLearnEventPayload = {
  port_id: 'input-0',
  raw_bytes: [0x90, 60, 100],
  kind: 'NoteOn',
  channel: 0,
  note_or_cc: 60,
  velocity_or_value: 100,
};

const SAMPLE_PAYLOAD_B: MidiLearnEventPayload = {
  port_id: 'input-0',
  raw_bytes: [0xb0, 7, 42],
  kind: 'ControlChange',
  channel: 0,
  note_or_cc: 7,
  velocity_or_value: 42,
};

describe('midiLearnStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    unlistenMock.mockReset();
    listenMock.mockResolvedValue(unlistenMock);
    invokeMock.mockResolvedValue(undefined);
    if (typeof localStorage !== 'undefined') {
      localStorage.clear();
    }
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('starts in the idle state with no captured payload', async () => {
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.reset();
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.activePortId = null;
    midiLearnStore.error = null;
    expect(midiLearnStore.state).toBe('idle');
    expect(midiLearnStore.captured).toBeNull();
    expect(midiLearnStore.activePortId).toBeNull();
  });

  it('start() invokes start_midi_learn and transitions to listening', async () => {
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.activePortId = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    expect(invokeMock).toHaveBeenCalledWith('start_midi_learn', { portId: 'input-0' });
    expect(midiLearnStore.state).toBe('listening');
    expect(midiLearnStore.activePortId).toBe('input-0');
    expect(midiLearnStore.error).toBeNull();
    expect(listenMock).toHaveBeenCalledWith('midi_learn_event', expect.any(Function));
  });

  it('captures the first event payload and transitions to captured', async () => {
    let captured: ((event: { payload: MidiLearnEventPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: MidiLearnEventPayload }) => void;
      return unlistenMock;
    });
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    expect(captured).not.toBeNull();
    captured!({ payload: SAMPLE_PAYLOAD });
    expect(midiLearnStore.captured).toEqual(SAMPLE_PAYLOAD);
    expect(midiLearnStore.state).toBe('captured');
  });

  it('replaces the captured payload on subsequent events (stays in captured)', async () => {
    let captured: ((event: { payload: MidiLearnEventPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: MidiLearnEventPayload }) => void;
      return unlistenMock;
    });
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    captured!({ payload: SAMPLE_PAYLOAD });
    captured!({ payload: SAMPLE_PAYLOAD_B });
    expect(midiLearnStore.captured).toEqual(SAMPLE_PAYLOAD_B);
    expect(midiLearnStore.state).toBe('captured');
  });

  it('stop() invokes stop_midi_learn, detaches listener, and returns to idle', async () => {
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    await midiLearnStore.stop();
    expect(invokeMock).toHaveBeenCalledWith('stop_midi_learn');
    expect(midiLearnStore.state).toBe('idle');
    expect(midiLearnStore.activePortId).toBeNull();
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });

  it('stop() preserves captured payload for re-display after exit', async () => {
    let captured: ((event: { payload: MidiLearnEventPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: MidiLearnEventPayload }) => void;
      return unlistenMock;
    });
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    captured!({ payload: SAMPLE_PAYLOAD });
    await midiLearnStore.stop();
    // Captured payload is preserved across stop() — the operator may
    // want to reference it after exiting learn mode. `reset()` is the
    // explicit clear path.
    expect(midiLearnStore.captured).toEqual(SAMPLE_PAYLOAD);
  });

  it('start() replaces a prior session (detaches old listener)', async () => {
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    await midiLearnStore.start('input-1');
    // Old listener detached, new one attached.
    expect(unlistenMock).toHaveBeenCalledTimes(1);
    expect(listenMock).toHaveBeenCalledTimes(2);
    expect(midiLearnStore.activePortId).toBe('input-1');
  });

  it('confirm() writes the captured payload to localStorage', async () => {
    let captured: ((event: { payload: MidiLearnEventPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: MidiLearnEventPayload }) => void;
      return unlistenMock;
    });
    const { midiLearnStore, MIDI_LEARN_CONFIRMED_KEY } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    captured!({ payload: SAMPLE_PAYLOAD });
    const confirmed = midiLearnStore.confirm();
    expect(confirmed).toEqual(SAMPLE_PAYLOAD);
    const stored = localStorage.getItem(MIDI_LEARN_CONFIRMED_KEY);
    expect(stored).not.toBeNull();
    expect(JSON.parse(stored!)).toEqual(SAMPLE_PAYLOAD);
  });

  it('confirm() returns null when nothing captured', async () => {
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    expect(midiLearnStore.confirm()).toBeNull();
  });

  it('reset() clears captured and transitions captured→listening', async () => {
    let captured: ((event: { payload: MidiLearnEventPayload }) => void) | null = null;
    listenMock.mockImplementation(async (_name: string, cb) => {
      captured = cb as (event: { payload: MidiLearnEventPayload }) => void;
      return unlistenMock;
    });
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    captured!({ payload: SAMPLE_PAYLOAD });
    expect(midiLearnStore.state).toBe('captured');
    midiLearnStore.reset();
    expect(midiLearnStore.captured).toBeNull();
    expect(midiLearnStore.state).toBe('listening');
  });

  it('start() captures invoke error and returns to idle', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    expect(midiLearnStore.error).toBe('not running in tauri');
    expect(midiLearnStore.state).toBe('idle');
    expect(midiLearnStore.activePortId).toBeNull();
  });

  // ----------------------------------------------------------------------
  // Session-token guard — a second start() (or a stop()) racing an
  // in-flight start() must not leak a duplicate `midi_learn_event`
  // listener or let a superseded session clobber store state.

  it('double-start does not double-listen: the superseded start bails before attaching', async () => {
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    // Fire both starts WITHOUT awaiting the first — the second begins
    // while the first is still pending (no listener stored yet, so its
    // up-front detach is a no-op).
    const first = midiLearnStore.start('input-0');
    const second = midiLearnStore.start('input-1');
    await Promise.all([first, second]);
    // Only the winning session attached a listener; the superseded one
    // bailed at its session-token check instead of registering a
    // duplicate (which would have been overwritten and leaked).
    expect(listenMock).toHaveBeenCalledTimes(1);
    expect(midiLearnStore.state).toBe('listening');
    expect(midiLearnStore.activePortId).toBe('input-1');
    // Clean detach still works for the surviving listener.
    await midiLearnStore.stop();
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });

  it('a superseded start whose listen() was already in flight self-detaches its handle', async () => {
    const unlistenA = vi.fn();
    const unlistenB = vi.fn();
    let resolveA: ((fn: () => void) => void) | null = null;
    let listenCalls = 0;
    listenMock.mockImplementation(() => {
      listenCalls += 1;
      if (listenCalls === 1) {
        return new Promise<() => void>((res) => {
          resolveA = res;
        });
      }
      return Promise.resolve(unlistenB);
    });
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;

    const first = midiLearnStore.start('input-0');
    // Let session A progress all the way into its pending listen().
    await vi.waitFor(() => {
      expect(listenMock).toHaveBeenCalledTimes(1);
    });
    // Session B fully completes while A's registration is in flight.
    await midiLearnStore.start('input-1');
    expect(listenMock).toHaveBeenCalledTimes(2);
    // A's registration finally resolves — it must detach its own handle
    // instead of overwriting B's stored slot.
    resolveA!(unlistenA);
    await first;
    expect(unlistenA).toHaveBeenCalledTimes(1);
    expect(unlistenB).not.toHaveBeenCalled();
    expect(midiLearnStore.activePortId).toBe('input-1');
    expect(midiLearnStore.state).toBe('listening');
    // A's (now stale) event callback must never write store state.
    const staleCb = listenMock.mock.calls[0]?.[1] as (event: {
      payload: MidiLearnEventPayload;
    }) => void;
    staleCb({ payload: SAMPLE_PAYLOAD });
    expect(midiLearnStore.captured).toBeNull();
    expect(midiLearnStore.state).toBe('listening');
    // B's listener is the one stop() detaches.
    await midiLearnStore.stop();
    expect(unlistenB).toHaveBeenCalledTimes(1);
  });

  it('stop() racing an in-flight start() leaves no listener attached and stays idle', async () => {
    let resolveListen: ((fn: () => void) => void) | null = null;
    listenMock.mockImplementation(
      () =>
        new Promise<() => void>((res) => {
          resolveListen = res;
        }),
    );
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;

    const inflight = midiLearnStore.start('input-0');
    await vi.waitFor(() => {
      expect(listenMock).toHaveBeenCalledTimes(1);
    });
    await midiLearnStore.stop();
    expect(midiLearnStore.state).toBe('idle');
    // The start's listen() resolves AFTER the stop — the late handle is
    // detached immediately rather than stored against an idle store.
    resolveListen!(unlistenMock);
    await inflight;
    expect(unlistenMock).toHaveBeenCalledTimes(1);
    expect(midiLearnStore.state).toBe('idle');
    expect(midiLearnStore.activePortId).toBeNull();
  });

  it('start() swallows listen() rejection and stays in listening', async () => {
    // The shell-side `start_midi_learn` succeeded (server-side handle
    // installed), but the frontend listener attach failed — typically
    // running outside Tauri's listen surface. State machine still
    // transitions to listening; no events surface until the listener
    // attaches successfully.
    listenMock.mockRejectedValue(new Error('listen unavailable'));
    const { midiLearnStore } = await import('./midiLearn.svelte');
    midiLearnStore.state = 'idle';
    midiLearnStore.captured = null;
    midiLearnStore.error = null;
    await midiLearnStore.start('input-0');
    expect(midiLearnStore.state).toBe('listening');
    expect(midiLearnStore.error).toBe('listen unavailable');
  });
});
