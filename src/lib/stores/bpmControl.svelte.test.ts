// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, it } from 'vitest';

import { bpmControlStore } from './bpmControl.svelte';

describe('bpmControlStore', () => {
  beforeEach(() => {
    bpmControlStore.reset();
  });

  it('toggles between auto-listening and manual locked BPM', () => {
    expect(bpmControlStore.isListening).toBe(true);
    expect(bpmControlStore.effectiveBpm(132.4)).toBe(132.4);

    bpmControlStore.toggleListening(140);
    expect(bpmControlStore.isListening).toBe(false);
    expect(bpmControlStore.manualBpm).toBe(140);
    // Ignores detector drift while locked
    expect(bpmControlStore.effectiveBpm(98)).toBe(140);

    bpmControlStore.toggleListening(98);
    expect(bpmControlStore.isListening).toBe(true);
    expect(bpmControlStore.effectiveBpm(98)).toBe(98);
  });

  it('scrubs and nudges beat phase cleanly within [-0.5, +0.5]', () => {
    bpmControlStore.setManualBpm(120);
    bpmControlStore.resyncDownbeat(1000);
    expect(bpmControlStore.beatPosition(1500, 120)).toBeCloseTo(1.0, 3);

    bpmControlStore.setPhaseOffset(0.25);
    expect(bpmControlStore.beatPosition(1500, 120)).toBeCloseTo(1.25, 3);

    bpmControlStore.nudgePhase(0.3);
    // Wraps +0.55 -> -0.45 beats
    expect(bpmControlStore.phaseOffsetBeats).toBeCloseTo(-0.45, 3);
  });

  it('computes manual BPM from tap-tempo intervals', () => {
    expect(bpmControlStore.tap(1000)).toBeNull();
    expect(bpmControlStore.tap(1500)).toBe(120);
    expect(bpmControlStore.tap(2000)).toBe(120);
    expect(bpmControlStore.mode).toBe('manual');
  });
});
