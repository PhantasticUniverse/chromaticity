// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W32 Xy primitive tests — exercises the normalised dot positioning
// against [-1, 1] and [0, 1] ranges, plus the readout formatting.
// Extended in parity-closure Lane D to cover pointer-drag interactivity,
// keyboard nudge, a11y branch switching, and snap-to-corner clamping.
//
// Pointer-event stubs mirror `Fader.test.ts` / `Knob.test.ts`:
// jsdom returns a 0×0 bounding rect by default + does not implement
// setPointerCapture / hasPointerCapture / releasePointerCapture, so
// both are stubbed on the pad element before firing events.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.
//
// Reduced-motion: use `vi.stubGlobal('matchMedia', ...)` +
// `vi.unstubAllGlobals()` — `vi.spyOn(window,'matchMedia')` throws in
// this jsdom stack per `feedback_vitest_matchmedia_stubglobal.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import Xy from './Xy.svelte';

function stubPadRect(pad: HTMLElement, width = 100, height = 100, left = 0, top = 0): void {
  pad.getBoundingClientRect = (): DOMRect =>
    ({
      x: left,
      y: top,
      left,
      top,
      right: left + width,
      bottom: top + height,
      width,
      height,
      toJSON: () => ({}),
    }) as DOMRect;
}

function stubPointerCapture(pad: HTMLElement): void {
  const captured = new Set<number>();
  pad.setPointerCapture = (pointerId: number) => {
    captured.add(pointerId);
  };
  pad.releasePointerCapture = (pointerId: number) => {
    captured.delete(pointerId);
  };
  pad.hasPointerCapture = (pointerId: number) => captured.has(pointerId);
}

describe('Xy widget', () => {
  afterEach(() => {
    cleanup();
  });

  // ---------------- W32 display contract (preserved) ---------------------

  it('centres the dot at 50%/50% for default-range zero value', () => {
    const { container } = render(Xy, { props: { value: [0, 0] } });
    const dot = container.querySelector('.xy__dot') as HTMLElement | null;
    expect(dot).not.toBeNull();
    if (dot === null) return;
    expect(dot.style.left).toBe('50%');
    expect(dot.style.top).toBe('50%');
  });

  it('positions the dot at top-right for max value', () => {
    const { container } = render(Xy, { props: { value: [1, 1] } });
    const dot = container.querySelector('.xy__dot') as HTMLElement | null;
    expect(dot).not.toBeNull();
    if (dot === null) return;
    expect(dot.style.left).toBe('100%');
    // y is inverted (visual top = max y).
    expect(dot.style.top).toBe('0%');
  });

  it('clamps positions outside the range to the pad edge', () => {
    const { container } = render(Xy, { props: { value: [10, -10] } });
    const dot = container.querySelector('.xy__dot') as HTMLElement | null;
    expect(dot).not.toBeNull();
    if (dot === null) return;
    expect(dot.style.left).toBe('100%');
    expect(dot.style.top).toBe('100%');
  });

  it('renders the readout values', () => {
    const { getByText } = render(Xy, { props: { value: [0.42, -0.5] } });
    expect(getByText('x 0.42')).toBeInTheDocument();
    expect(getByText('y -0.50')).toBeInTheDocument();
  });

  it('dot tracks the value with no inline transition (FIX 2)', () => {
    // FIX 2 removed `transition: left 40ms linear, top 40ms linear` from
    // `.xy__dot` so live drags track 1:1 instead of lagging 40ms behind
    // the pointer. jsdom can't surface the scoped <style> rule via
    // getComputedStyle, so assert nothing re-adds an inline transition
    // on the moving dot (mirrors the Meter.test.ts FIX 2 regression).
    const { container } = render(Xy, {
      props: { value: [0.25, -0.25], onchange: () => {} },
    });
    const dot = container.querySelector('.xy__dot') as HTMLElement | null;
    expect(dot).not.toBeNull();
    if (dot === null) return;
    expect(dot.style.transition).toBe('');
  });

  it('respects a [0, 1] explicit range', () => {
    const { container } = render(Xy, { props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1] } });
    const dot = container.querySelector('.xy__dot') as HTMLElement | null;
    expect(dot).not.toBeNull();
    if (dot === null) return;
    expect(dot.style.left).toBe('50%');
    expect(dot.style.top).toBe('50%');
  });

  // ---------------- Lane D a11y branch switching -------------------------

  it('renders role="img" when no onchange callback is provided', () => {
    const { container } = render(Xy, { props: { value: [0, 0] } });
    const pad = container.querySelector('.xy__pad');
    expect(pad?.getAttribute('role')).toBe('img');
    expect(pad?.getAttribute('aria-valuenow')).toBeNull();
    expect(pad?.getAttribute('tabindex')).toBeNull();
  });

  it('renders role="slider" + aria-value* when onchange is provided', () => {
    const onchange = vi.fn();
    const { container } = render(Xy, { props: { value: [0, 0], onchange } });
    const pad = container.querySelector('.xy__pad');
    expect(pad?.getAttribute('role')).toBe('slider');
    expect(pad?.getAttribute('tabindex')).toBe('0');
    expect(pad?.getAttribute('aria-valuemin')).toBe('0');
    expect(pad?.getAttribute('aria-valuemax')).toBe('1');
    // X=0, range [-1,1]: (0 - (-1)) / 2 = 0.5.
    expect(Number(pad?.getAttribute('aria-valuenow'))).toBeCloseTo(0.5, 5);
  });

  it('exposes aria-roledescription "xy pad slider" on the interactive pad', () => {
    const onchange = vi.fn();
    const { container } = render(Xy, { props: { value: [0, 0], onchange } });
    const pad = container.querySelector('.xy__pad');
    expect(pad?.getAttribute('aria-roledescription')).toBe('xy pad slider');
  });

  it('includes axis values in aria-label', () => {
    const onchange = vi.fn();
    const { container } = render(Xy, { props: { value: [0.5, -0.5], onchange } });
    const pad = container.querySelector('.xy__pad');
    const label = pad?.getAttribute('aria-label') ?? '';
    expect(label).toContain('x 0.50');
    expect(label).toContain('y -0.50');
  });

  // ---------------- Lane D pointer-drag interactivity --------------------

  it('pointerdown on the pad fires onchange with the client-derived position', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    stubPadRect(pad, 100, 100, 0, 0);
    stubPointerCapture(pad);
    // clientX=25, clientY=25 → nx=0.25, ny inverted=0.75.
    await fireEvent.pointerDown(pad, { clientX: 25, clientY: 25, pointerId: 1 });
    expect(onchange).toHaveBeenCalledTimes(1);
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.25, 5);
    expect(call?.[1]).toBeCloseTo(0.75, 5);
  });

  it('pointermove after pointerdown fires onchange with updated values', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    stubPadRect(pad, 100, 100, 0, 0);
    stubPointerCapture(pad);
    await fireEvent.pointerDown(pad, { clientX: 50, clientY: 50, pointerId: 1 });
    onchange.mockClear();
    // Move to (80, 20): nx=0.8, ny inverted=0.8.
    await fireEvent.pointerMove(pad, { clientX: 80, clientY: 20, pointerId: 1 });
    expect(onchange).toHaveBeenCalledTimes(1);
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.8, 5);
    expect(call?.[1]).toBeCloseTo(0.8, 5);
  });

  it('pointermove without prior pointerdown is a no-op (drag gate)', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    stubPadRect(pad, 100, 100, 0, 0);
    await fireEvent.pointerMove(pad, { clientX: 50, clientY: 50, pointerId: 1 });
    expect(onchange).not.toHaveBeenCalled();
  });

  it('clamps drag position to pad edges (outside-rect drag)', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    stubPadRect(pad, 100, 100, 0, 0);
    stubPointerCapture(pad);
    // clientX=200, clientY=-50 (far outside pad).
    await fireEvent.pointerDown(pad, { clientX: 200, clientY: -50, pointerId: 1 });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(1, 5);
    expect(call?.[1]).toBeCloseTo(1, 5);
  });

  it('disabled=true suppresses pointerdown and adds disabled class', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange, disabled: true },
    });
    const root = container.querySelector('.xy');
    expect(root?.classList.contains('xy--disabled')).toBe(true);
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    expect(pad.getAttribute('aria-disabled')).toBe('true');
    stubPadRect(pad, 100, 100, 0, 0);
    stubPointerCapture(pad);
    await fireEvent.pointerDown(pad, { clientX: 50, clientY: 50, pointerId: 1 });
    expect(onchange).not.toHaveBeenCalled();
  });

  // ---------------- Lane D keyboard navigation ---------------------------

  it('ArrowRight nudges X up by 5% of range', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowRight' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.05, 5);
    expect(call?.[1]).toBeCloseTo(0, 5);
  });

  it('ArrowLeft nudges X down by 5% of range', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowLeft' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.45, 5);
    expect(call?.[1]).toBeCloseTo(0.5, 5);
  });

  it('ArrowUp nudges Y up by 5% of range', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowUp' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.5, 5);
    expect(call?.[1]).toBeCloseTo(0.55, 5);
  });

  it('ArrowDown nudges Y down by 5% of range', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowDown' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.5, 5);
    expect(call?.[1]).toBeCloseTo(0.45, 5);
  });

  it('Shift+ArrowRight uses the 20% large step on X', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.1, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowRight', shiftKey: true });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.3, 5);
    expect(call?.[1]).toBeCloseTo(0.5, 5);
  });

  it('Alt+ArrowRight nudges both axes diagonally', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowRight', altKey: true });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.55, 5);
    expect(call?.[1]).toBeCloseTo(0.55, 5);
  });

  it('Alt+ArrowLeft nudges both axes diagonally (decrease)', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowLeft', altKey: true });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0.45, 5);
    expect(call?.[1]).toBeCloseTo(0.45, 5);
  });

  it('Home snaps to (min, min)', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'Home' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0, 5);
    expect(call?.[1]).toBeCloseTo(0, 5);
  });

  it('End snaps to (max, max)', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'End' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(1, 5);
    expect(call?.[1]).toBeCloseTo(1, 5);
  });

  it('clamps ArrowUp at the max edge', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.98], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowUp' });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[1]).toBeCloseTo(1, 5);
  });

  it('disabled pad does not fire onchange on keydown', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1], onchange, disabled: true },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    if (pad === null) throw new Error('xy__pad not rendered');
    await fireEvent.keyDown(pad, { key: 'ArrowRight' });
    expect(onchange).not.toHaveBeenCalled();
  });

  // ---------------- Lane D snap-to-corner --------------------------------

  it('snaps to min edge when drag is within the snap zone', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    stubPadRect(pad, 100, 100, 0, 0);
    stubPointerCapture(pad);
    // clientX=3, clientY=97 → nx=0.03 (< SNAP_ZONE 0.05) → snaps to 0;
    // ny raw = 1 - 97/100 = 0.03 → snaps to 0 (bottom = min Y).
    await fireEvent.pointerDown(pad, { clientX: 3, clientY: 97, pointerId: 1 });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(0, 5);
    expect(call?.[1]).toBeCloseTo(0, 5);
  });

  it('snaps to max edge when drag is near the max corner', async () => {
    const onchange = vi.fn();
    const { container } = render(Xy, {
      props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
    });
    const pad = container.querySelector('.xy__pad') as HTMLElement | null;
    expect(pad).not.toBeNull();
    if (pad === null) return;
    stubPadRect(pad, 100, 100, 0, 0);
    stubPointerCapture(pad);
    // clientX=98 → nx=0.98 (> 1 - SNAP_ZONE 0.95) → snaps to 1;
    // clientY=3 → ny raw = 1 - 0.03 = 0.97 → snaps to 1 (top = max Y).
    await fireEvent.pointerDown(pad, { clientX: 98, clientY: 3, pointerId: 1 });
    const call = onchange.mock.calls[0]?.[0] as [number, number] | undefined;
    expect(call?.[0]).toBeCloseTo(1, 5);
    expect(call?.[1]).toBeCloseTo(1, 5);
  });

  // ---------------- Lane D reduced-motion --------------------------------

  it('renders without errors under reduced-motion matchMedia stub', () => {
    vi.stubGlobal('matchMedia', (query: string) => ({
      matches: query.includes('prefers-reduced-motion'),
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      dispatchEvent: () => false,
    }));
    try {
      const onchange = vi.fn();
      const { container } = render(Xy, {
        props: { value: [0, 0], min: [0, 0], max: [1, 1], onchange },
      });
      expect(container.querySelector('.xy__pad')).not.toBeNull();
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
