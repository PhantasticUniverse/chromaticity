// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, it } from 'vitest';

import { djAutopilotStore } from './djAutopilot.svelte';
import { inspectAllStateMachines, registeredStateMachineNames } from '../state/diagnostics';

describe('djAutopilotStore', () => {
  beforeEach(() => {
    djAutopilotStore.reset();
  });

  it('starts in the off state and registers with the diagnostics registry', () => {
    expect(djAutopilotStore.state).toEqual({ kind: 'off' });
    expect(djAutopilotStore.isActive).toBe(false);
    expect(registeredStateMachineNames()).toContain('djAutopilot');
    expect(inspectAllStateMachines().djAutopilot).toEqual({
      kind: 'off',
      phraseIntervalBars: null,
      subtleDrift: false,
      crossfadeMs: 0,
    });
  });

  it('toggles between off and active with default settings', () => {
    djAutopilotStore.toggle();
    expect(djAutopilotStore.isActive).toBe(true);
    expect(djAutopilotStore.state).toEqual({
      kind: 'active',
      phraseIntervalBars: 16,
      subtleDrift: true,
      crossfadeMs: 1200,
    });
    expect(inspectAllStateMachines().djAutopilot).toEqual({
      kind: 'active',
      phraseIntervalBars: 16,
      subtleDrift: true,
      crossfadeMs: 1200,
    });

    djAutopilotStore.toggle();
    expect(djAutopilotStore.isActive).toBe(false);
    expect(djAutopilotStore.state).toEqual({ kind: 'off' });
  });

  it('supports enable with custom options, clamping crossfadeMs, and disable', () => {
    djAutopilotStore.enable({
      phraseIntervalBars: 32,
      subtleDrift: false,
      crossfadeMs: 2400,
    });
    expect(djAutopilotStore.state).toEqual({
      kind: 'active',
      phraseIntervalBars: 32,
      subtleDrift: false,
      crossfadeMs: 2400,
    });

    djAutopilotStore.enable({ crossfadeMs: 25_000 });
    expect(djAutopilotStore.crossfadeMs).toBe(10_000);

    djAutopilotStore.enable({ crossfadeMs: -50 });
    expect(djAutopilotStore.crossfadeMs).toBe(0);

    djAutopilotStore.disable();
    expect(djAutopilotStore.state).toEqual({ kind: 'off' });
  });

  it('updates phraseIntervalBars and subtleDrift while active and preserves preferences across toggle', () => {
    djAutopilotStore.enable();
    djAutopilotStore.setPhraseInterval(8);
    djAutopilotStore.setSubtleDrift(false);

    expect(djAutopilotStore.state).toEqual({
      kind: 'active',
      phraseIntervalBars: 8,
      subtleDrift: false,
      crossfadeMs: 1200,
    });

    djAutopilotStore.disable();
    expect(djAutopilotStore.state).toEqual({ kind: 'off' });

    djAutopilotStore.toggle();
    expect(djAutopilotStore.state).toEqual({
      kind: 'active',
      phraseIntervalBars: 8,
      subtleDrift: false,
      crossfadeMs: 1200,
    });
  });

  it('resets active state and custom preferences back to factory defaults', () => {
    djAutopilotStore.enable({
      phraseIntervalBars: 8,
      subtleDrift: false,
      crossfadeMs: 500,
    });

    djAutopilotStore.reset();
    expect(djAutopilotStore.state).toEqual({ kind: 'off' });

    djAutopilotStore.toggle();
    expect(djAutopilotStore.state).toEqual({
      kind: 'active',
      phraseIntervalBars: 16,
      subtleDrift: true,
      crossfadeMs: 1200,
    });
  });
});
