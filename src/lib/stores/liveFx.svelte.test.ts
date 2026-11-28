// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { liveFxStore } from './liveFx.svelte';
import { inspectAllStateMachines } from '../state/diagnostics';

describe('liveFxStore state machine', () => {
  beforeEach(() => {
    liveFxStore.reset();
  });

  afterEach(() => {
    liveFxStore.reset();
  });

  it('handles normal <-> strobe and idempotent activate/release transitions', () => {
    expect(liveFxStore.state).toEqual({ kind: 'normal' });
    expect(liveFxStore.activateStrobe()).toBe(true);
    expect(liveFxStore.state).toEqual({ kind: 'strobe' });
    expect(liveFxStore.effectiveStrobeVisible).toBe(true);

    // Re-activating strobe while already active returns false.
    expect(liveFxStore.activateStrobe()).toBe(false);
    expect(liveFxStore.releaseStrobe()).toBe(true);
    expect(liveFxStore.state).toEqual({ kind: 'normal' });
    expect(liveFxStore.releaseStrobe()).toBe(false);
  });

  it('tracks strobeHeld cleanly across blackout toggle without losing release state', () => {
    liveFxStore.activateStrobe();
    expect(liveFxStore.toggleBlackout()).toBe(true);
    expect(liveFxStore.state).toEqual({ kind: 'blackout', strobeHeld: true });
    expect(liveFxStore.blackoutActive).toBe(true);
    expect(liveFxStore.strobeActive).toBe(true);
    expect(liveFxStore.effectiveStrobeVisible).toBe(false);

    // Release strobe while blacked out, then exit blackout -> returns cleanly to normal.
    expect(liveFxStore.releaseStrobe()).toBe(true);
    expect(liveFxStore.state).toEqual({ kind: 'blackout', strobeHeld: false });
    expect(liveFxStore.toggleBlackout()).toBe(false);
    expect(liveFxStore.state).toEqual({ kind: 'normal' });
  });

  it('clears both blackout and strobe on reset and reports diagnostics accurately', () => {
    liveFxStore.toggleBlackout();
    liveFxStore.activateStrobe();

    expect(inspectAllStateMachines().liveFx).toEqual({
      kind: 'blackout',
      blackoutActive: true,
      strobeActive: true,
    });

    const cleared = liveFxStore.reset();
    expect(cleared).toEqual({ releasedStrobe: true, clearedBlackout: true });
    expect(liveFxStore.state).toEqual({ kind: 'normal' });
    expect(inspectAllStateMachines().liveFx).toEqual({
      kind: 'normal',
      blackoutActive: false,
      strobeActive: false,
    });
  });
});
