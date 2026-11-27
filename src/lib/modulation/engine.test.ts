// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const setControlMock = vi.fn<(...args: [number, number]) => Promise<void>>(() => Promise.resolve());

vi.mock('../ipc', () => ({
  setControl: (...args: [number, number]) => setControlMock(...args),
}));

import {
  evaluateModulationGraph,
  extractAudioInputs,
  sampleModSource,
  type ModRoute,
} from './engine';
import { DEFAULT_MOD_ROUTES, PatchModulationStore, patchModulationStore } from './store.svelte';
import { inspectAllStateMachines } from '../state';
import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

function makeRoute(overrides: Partial<ModRoute> = {}): ModRoute {
  return {
    id: 'r1',
    source: 'audio.rms',
    targetControlIndex: 0,
    op: 'passthrough',
    amount: 1,
    bias: 0,
    smoothing: 0,
    enabled: true,
    ...overrides,
  };
}

describe('modulation engine (pure)', () => {
  it('evaluates LFO sine, triangle, and saw waveforms deterministically', () => {
    // At 1 Hz, elapsedSecs maps 1:1 to phase in [0, 1).
    expect(sampleModSource('lfo.sine', { elapsedSecs: 0, lfoRateHz: 1 })).toBeCloseTo(0.5, 5);
    expect(sampleModSource('lfo.sine', { elapsedSecs: 0.25, lfoRateHz: 1 })).toBeCloseTo(1.0, 5);
    expect(sampleModSource('lfo.sine', { elapsedSecs: 0.5, lfoRateHz: 1 })).toBeCloseTo(0.5, 5);
    expect(sampleModSource('lfo.sine', { elapsedSecs: 0.75, lfoRateHz: 1 })).toBeCloseTo(0.0, 5);

    expect(sampleModSource('lfo.triangle', { elapsedSecs: 0, lfoRateHz: 1 })).toBeCloseTo(0.0, 5);
    expect(sampleModSource('lfo.triangle', { elapsedSecs: 0.25, lfoRateHz: 1 })).toBeCloseTo(
      0.5,
      5,
    );
    expect(sampleModSource('lfo.triangle', { elapsedSecs: 0.5, lfoRateHz: 1 })).toBeCloseTo(1.0, 5);
    expect(sampleModSource('lfo.triangle', { elapsedSecs: 0.75, lfoRateHz: 1 })).toBeCloseTo(
      0.5,
      5,
    );

    expect(sampleModSource('lfo.saw', { elapsedSecs: 0, lfoRateHz: 1 })).toBeCloseTo(0.0, 5);
    expect(sampleModSource('lfo.saw', { elapsedSecs: 0.4, lfoRateHz: 1 })).toBeCloseTo(0.4, 5);
    expect(sampleModSource('lfo.saw', { elapsedSecs: 1.4, lfoRateHz: 1 })).toBeCloseTo(0.4, 5);
  });

  it('routes audio sources including normalized BPM and FFT band averages', () => {
    const audio = extractAudioInputs({
      bpm: 120,
      rms: 0.6,
      peak: 0.9,
      fft_bands: [0.8, 0.4, 0.3, 0.6, 0.9, 0.2, 0.4, 0.6],
    });
    expect(audio.bass).toBeCloseTo(0.6, 5);
    expect(audio.mid).toBeCloseTo(0.6, 5);
    expect(audio.high).toBeCloseTo(0.4, 5);

    const routes: ModRoute[] = [
      makeRoute({ id: 'a', source: 'audio.rms', targetControlIndex: 0 }),
      makeRoute({ id: 'b', source: 'audio.peak', targetControlIndex: 1 }),
      makeRoute({ id: 'c', source: 'audio.bass', targetControlIndex: 2 }),
      makeRoute({ id: 'd', source: 'audio.bpm', targetControlIndex: 3 }),
    ];
    const res = evaluateModulationGraph(routes, { elapsedSecs: 0, audio }, new Map());
    expect(res.outputs.get(0)).toBeCloseTo(0.6, 5);
    expect(res.outputs.get(1)).toBeCloseTo(0.9, 5);
    expect(res.outputs.get(2)).toBeCloseTo(0.6, 5);
    expect(res.outputs.get(3)).toBeCloseTo(0.5, 5);
  });

  it('applies gain, invert, quantize, and EMA smooth operations', () => {
    const gainRoute = makeRoute({
      id: 'g',
      source: 'audio.rms',
      targetControlIndex: 0,
      op: 'gain',
      amount: 1.5,
      bias: 0.1,
    });
    const invertRoute = makeRoute({
      id: 'inv',
      source: 'audio.rms',
      targetControlIndex: 1,
      op: 'invert',
      amount: 1,
      bias: 0,
    });
    const quantRoute = makeRoute({
      id: 'q',
      source: 'audio.rms',
      targetControlIndex: 2,
      op: 'quantize',
      amount: 1,
      bias: 0,
    });
    const smoothRoute = makeRoute({
      id: 's',
      source: 'audio.rms',
      targetControlIndex: 3,
      op: 'smooth',
      amount: 1,
      bias: 0,
      smoothing: 0.75,
    });

    const prevState = new Map<string, number>([['s', 0.0]]);
    const res = evaluateModulationGraph(
      [gainRoute, invertRoute, quantRoute, smoothRoute],
      { elapsedSecs: 0, audio: { rms: 0.4 } },
      prevState,
    );

    // gain: 0.4 * 1.5 + 0.1 = 0.7
    expect(res.outputs.get(0)).toBeCloseTo(0.7, 5);
    // invert: (1 - 0.4) * 1 = 0.6
    expect(res.outputs.get(1)).toBeCloseTo(0.6, 5);
    // quantize (4 steps): round(0.4 * 4) / 4 = 2 / 4 = 0.5
    expect(res.outputs.get(2)).toBeCloseTo(0.5, 5);
    // smooth: 0.0 * 0.75 + 0.4 * 0.25 = 0.1
    expect(res.outputs.get(3)).toBeCloseTo(0.1, 5);
    expect(res.nextState.get('s')).toBeCloseTo(0.1, 5);
  });

  it('sums multiple routes targeting the same control and clamps to [0, 1]', () => {
    const routes: ModRoute[] = [
      makeRoute({ id: 'r1', source: 'audio.bass', targetControlIndex: 0, op: 'gain', amount: 0.8 }),
      makeRoute({ id: 'r2', source: 'audio.rms', targetControlIndex: 0, op: 'gain', amount: 0.9 }),
      makeRoute({
        id: 'r3',
        source: 'audio.rms',
        targetControlIndex: 1,
        op: 'gain',
        amount: 0.2,
        bias: -0.5,
      }),
    ];
    const res = evaluateModulationGraph(
      routes,
      { elapsedSecs: 0, audio: { bass: 0.9, rms: 0.8 } },
      new Map(),
    );
    // 0.72 + 0.72 = 1.44 -> clamped to 1.0
    expect(res.outputs.get(0)).toBe(1);
    // 0.16 - 0.5 = -0.34 -> clamped to 0.0
    expect(res.outputs.get(1)).toBe(0);
  });

  it('ignores disabled routes and out-of-range control indices', () => {
    const routes: ModRoute[] = [
      makeRoute({ id: 'off', source: 'audio.rms', targetControlIndex: 0, enabled: false }),
      makeRoute({ id: 'bad', source: 'audio.rms', targetControlIndex: 9, enabled: true }),
    ];
    const res = evaluateModulationGraph(routes, { elapsedSecs: 0, audio: { rms: 0.8 } }, new Map());
    expect(res.outputs.size).toBe(0);
    expect(res.routeValues.size).toBe(0);
  });
});

describe('PatchModulationStore lifecycle', () => {
  beforeEach(() => {
    setControlMock.mockClear();
    localStorage.clear();
    audioSnapshotStore.snapshot = null;
    patchModulationStore.stop();
    patchModulationStore.resetDefaults();
  });

  afterEach(() => {
    patchModulationStore.stop();
    audioSnapshotStore.snapshot = null;
  });

  it('seeds with 3 musical default routes and registers in diagnostics', () => {
    expect(DEFAULT_MOD_ROUTES.length).toBe(3);
    expect(patchModulationStore.routes.length).toBe(3);
    const diag = inspectAllStateMachines();
    expect(diag.patchModulation).toEqual({
      kind: 'stopped',
      tickCount: 0,
      routeCount: 3,
      enabledRouteCount: 3,
    });
  });

  it('supports start, step dispatch, stop, addRoute, toggleRoute, and removeRoute', () => {
    const store = new PatchModulationStore();
    expect(store.machine.kind).toBe('stopped');

    const added = store.addRoute({
      source: 'lfo.saw',
      targetControlIndex: 4,
      op: 'gain',
      amount: 1,
    });
    expect(store.routes.some((r) => r.id === added.id)).toBe(true);

    store.toggleRoute(added.id);
    expect(store.routes.find((r) => r.id === added.id)?.enabled).toBe(false);

    store.start();
    expect(store.machine).toEqual({ kind: 'running', tickCount: 0 });

    audioSnapshotStore.snapshot = {
      bpm: 120,
      bpm_availability: 'Ready',
      rms: 0.5,
      peak: 0.7,
      fft_bands: [0.6, 0.6, 0.2, 0.2, 0.2, 0.1, 0.1, 0.1],
      fft_availability: 'Ready',
      degraded: false,
    };

    const out = store.step(0.5);
    expect(store.machine).toEqual({ kind: 'running', tickCount: 1 });
    expect(out.get(0)).toBeCloseTo(0.72, 2);
    expect(setControlMock).toHaveBeenCalled();

    // Stepping again with identical inputs should not re-dispatch unchanged controls.
    setControlMock.mockClear();
    store.step(0.5);
    expect(setControlMock).not.toHaveBeenCalled();

    store.removeRoute(added.id);
    expect(store.routes.some((r) => r.id === added.id)).toBe(false);

    store.stop();
    expect(store.machine).toEqual({ kind: 'stopped' });
  });
});
