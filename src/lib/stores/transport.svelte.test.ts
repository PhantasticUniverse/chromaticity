// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane B.2 — transport clock store unit tests.
//
// The store wraps `performance.now()` via a `requestAnimationFrame` tick
// loop. Under jsdom the rAF schedule fires on real wall-clock cadence
// which is too coarse for deterministic millisecond-level assertions;
// these tests therefore stub `requestAnimationFrame` /
// `cancelAnimationFrame` / `performance.now()` so the tick can be
// advanced deterministically frame-by-frame.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { transportStore } from './transport.svelte';

describe('transportStore', () => {
  let nowMs = 0;
  let pendingTick: FrameRequestCallback | null = null;
  let rafHandle = 1;

  beforeEach(() => {
    nowMs = 0;
    pendingTick = null;
    rafHandle = 1;

    vi.stubGlobal('performance', {
      now: () => nowMs,
    });
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback): number => {
      pendingTick = cb;
      return rafHandle++;
    });
    vi.stubGlobal('cancelAnimationFrame', (): void => {
      pendingTick = null;
    });
  });

  afterEach(() => {
    // Detach the store from the stubbed scheduler before unstubbing so
    // a lingering rAF handle does not survive into a sibling test file.
    transportStore.stop();
    transportStore.reset();
    vi.unstubAllGlobals();
  });

  /** Advance the stubbed clock by `dtMs` and fire the pending rAF
   * callback (if any) with the new `performance.now()` reading. */
  function advance(dtMs: number): void {
    nowMs += dtMs;
    const cb = pendingTick;
    pendingTick = null;
    if (cb) cb(nowMs);
  }

  it('starts with elapsedMs at 0 and started=false', () => {
    expect(transportStore.elapsedMs).toBe(0);
    expect(transportStore.started).toBe(false);
  });

  it('start() flips started=true and ticks elapsedMs against performance.now()', () => {
    transportStore.start();
    expect(transportStore.started).toBe(true);
    advance(100);
    expect(transportStore.elapsedMs).toBe(100);
    advance(50);
    expect(transportStore.elapsedMs).toBe(150);
  });

  it('start() is idempotent: second call is a no-op', () => {
    transportStore.start();
    advance(100);
    expect(transportStore.elapsedMs).toBe(100);
    // Re-calling start() must not re-anchor the clock back to zero.
    transportStore.start();
    advance(50);
    expect(transportStore.elapsedMs).toBe(150);
  });

  it('stop() halts the tick but preserves elapsedMs', () => {
    transportStore.start();
    advance(200);
    expect(transportStore.elapsedMs).toBe(200);
    transportStore.stop();
    expect(transportStore.started).toBe(false);
    // No further ticks should fire — pendingTick was cleared.
    expect(pendingTick).toBeNull();
    expect(transportStore.elapsedMs).toBe(200);
  });

  it('start() after stop() resumes from the paused position', () => {
    transportStore.start();
    advance(300);
    expect(transportStore.elapsedMs).toBe(300);
    transportStore.stop();
    // Wall-clock advances while paused — the store should NOT credit
    // that span when resumed.
    nowMs += 1000;
    transportStore.start();
    advance(100);
    expect(transportStore.elapsedMs).toBe(400);
  });

  it('reset() zeros elapsedMs while running', () => {
    transportStore.start();
    advance(500);
    transportStore.reset();
    expect(transportStore.elapsedMs).toBe(0);
    advance(100);
    expect(transportStore.elapsedMs).toBe(100);
  });

  it('reset() while stopped leaves the store at zero', () => {
    transportStore.start();
    advance(500);
    transportStore.stop();
    transportStore.reset();
    expect(transportStore.elapsedMs).toBe(0);
  });

  describe('timecode()', () => {
    it('formats zero elapsedMs as 00:00:00:00', () => {
      expect(transportStore.timecode()).toBe('00:00:00:00');
    });

    it('formats one second as 00:00:01:00', () => {
      transportStore.start();
      advance(1000);
      expect(transportStore.timecode()).toBe('00:00:01:00');
    });

    it('formats one minute thirty seconds five frames at 30fps', () => {
      transportStore.start();
      // 1 min 30 sec + 5 frames at 30fps = 90000 ms + (5 / 30) * 1000 ms
      advance(90_000 + (5 / 30) * 1000);
      expect(transportStore.timecode()).toBe('00:01:30:05');
    });

    it('formats one hour two minutes three seconds', () => {
      transportStore.start();
      advance(3600_000 + 2 * 60_000 + 3_000);
      expect(transportStore.timecode()).toBe('01:02:03:00');
    });

    it('zero-pads each segment to two digits', () => {
      transportStore.start();
      advance(7_000);
      expect(transportStore.timecode()).toBe('00:00:07:00');
    });

    it('FF wraps at 30 (frame index modulo FPS)', () => {
      transportStore.start();
      // 29 frames at 30fps = (29 / 30) * 1000 ms ≈ 966.67 ms; floor gives 29.
      advance((29 / 30) * 1000);
      expect(transportStore.timecode()).toBe('00:00:00:29');
      // Cross the second boundary — FF resets and SS increments.
      advance((1 / 30) * 1000 + 1);
      expect(transportStore.timecode()).toBe('00:00:01:00');
    });
  });
});
