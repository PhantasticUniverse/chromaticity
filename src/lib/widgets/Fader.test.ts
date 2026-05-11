// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W37 Fader widget tests — extended in Wave 2.1 Lane A.2 to cover
// pointer-drag → `onchange` dispatch. The W8 baseline contract (value
// → fill-height mapping, label / mod-halo render) is preserved; the
// W2.1 A.2 axis (pointer-drag interactivity) gates on the optional
// `onchange` callback presence (display-only callers like Inputs.svelte
// pass no onchange and stay non-interactive).
//
// Pointer-event stubs mirror the W46 `CrossFader.test.ts` precedent:
// jsdom returns a 0×0 bounding rect by default + does not implement
// setPointerCapture / hasPointerCapture / releasePointerCapture, so
// both are stubbed on the track element before firing events.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import Fader from './Fader.svelte';

function stubTrackRect(track: HTMLElement, height = 100, top = 0): void {
  track.getBoundingClientRect = (): DOMRect =>
    ({
      x: 0,
      y: top,
      left: 0,
      top,
      right: 10,
      bottom: top + height,
      width: 10,
      height,
      toJSON: () => ({}),
    }) as DOMRect;
}

function stubPointerCapture(track: HTMLElement): void {
  const captured = new Set<number>();
  track.setPointerCapture = (pointerId: number) => {
    captured.add(pointerId);
  };
  track.releasePointerCapture = (pointerId: number) => {
    captured.delete(pointerId);
  };
  track.hasPointerCapture = (pointerId: number) => captured.has(pointerId);
}

describe('Fader widget', () => {
  afterEach(() => {
    cleanup();
  });

  // ---------------- W8 display contract (preserved) ----------------------

  it('renders the track + fill + cap structure', () => {
    const { container } = render(Fader, { props: { value: 0.5 } });
    expect(container.querySelector('.fader')).not.toBeNull();
    expect(container.querySelector('.fader-track')).not.toBeNull();
    expect(container.querySelector('.fader-fill')).not.toBeNull();
    expect(container.querySelector('.fader-cap')).not.toBeNull();
  });

  it('translates value=0.5 into a 50% fill height', () => {
    const { container } = render(Fader, { props: { value: 0.5 } });
    const fill = container.querySelector('.fader-fill') as HTMLElement | null;
    expect(fill?.getAttribute('style')).toContain('height: 50%');
  });

  it('clamps values above 1 to 100% fill', () => {
    const { container } = render(Fader, { props: { value: 1.7 } });
    const fill = container.querySelector('.fader-fill') as HTMLElement | null;
    expect(fill?.getAttribute('style')).toContain('height: 100%');
  });

  it('clamps negative values to 0% fill', () => {
    const { container } = render(Fader, { props: { value: -0.4 } });
    const fill = container.querySelector('.fader-fill') as HTMLElement | null;
    expect(fill?.getAttribute('style')).toContain('height: 0%');
  });

  it('renders a label when the prop is provided', () => {
    const { getByText } = render(Fader, { props: { value: 0.5, label: 'gain' } });
    expect(getByText('gain')).toBeInTheDocument();
  });

  it('omits the label DOM when no label prop is given', () => {
    const { container } = render(Fader, { props: { value: 0.5 } });
    expect(container.querySelector('.fader-label')).toBeNull();
  });

  it('applies the `mod-halo` class when modulated=true', () => {
    const { container } = render(Fader, { props: { value: 0.5, modulated: true } });
    const root = container.querySelector('.fader');
    expect(root?.classList.contains('mod-halo')).toBe(true);
  });

  it('omits the `mod-halo` class by default', () => {
    const { container } = render(Fader, { props: { value: 0.5 } });
    const root = container.querySelector('.fader');
    expect(root?.classList.contains('mod-halo')).toBe(false);
  });

  // ---------------- Wave 2.1 Lane A.2 interactivity ---------------------

  it('stays role="presentation" when no onchange callback is provided', () => {
    const { container } = render(Fader, { props: { value: 0.5 } });
    const track = container.querySelector('.fader-track');
    expect(track?.getAttribute('role')).toBe('presentation');
    expect(track?.getAttribute('aria-valuenow')).toBeNull();
  });

  it('exposes role="slider" + aria-value* when onchange is provided', () => {
    const onchange = vi.fn();
    const { container } = render(Fader, { props: { value: 0.5, onchange } });
    const track = container.querySelector('.fader-track');
    expect(track?.getAttribute('role')).toBe('slider');
    expect(track?.getAttribute('aria-valuemin')).toBe('0');
    expect(track?.getAttribute('aria-valuemax')).toBe('1');
    expect(track?.getAttribute('aria-valuenow')).toBe('0.5');
  });

  it('pointerdown on the track fires onchange with the clientY-derived value', async () => {
    const onchange = vi.fn();
    const { container } = render(Fader, { props: { value: 0.5, onchange } });
    const track = container.querySelector('.fader-track') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 100, 0);
    stubPointerCapture(track);
    // clientY=25, rect.top=0, height=100 → raw = 1 - 25/100 = 0.75.
    await fireEvent.pointerDown(track, { clientX: 5, clientY: 25, pointerId: 1 });
    expect(onchange).toHaveBeenCalledWith(0.75);
  });

  it('pointermove after pointerdown fires onchange with updated values', async () => {
    const onchange = vi.fn();
    const { container } = render(Fader, { props: { value: 0.5, onchange } });
    const track = container.querySelector('.fader-track') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 100, 0);
    stubPointerCapture(track);
    await fireEvent.pointerDown(track, { clientX: 5, clientY: 50, pointerId: 1 });
    onchange.mockClear();
    // clientY=10 → raw = 1 - 10/100 = 0.9
    await fireEvent.pointerMove(track, { clientX: 5, clientY: 10, pointerId: 1 });
    // clientY=80 → raw = 1 - 80/100 ≈ 0.2 (float arithmetic).
    await fireEvent.pointerMove(track, { clientX: 5, clientY: 80, pointerId: 1 });
    expect(onchange).toHaveBeenCalledTimes(2);
    const firstCall = onchange.mock.calls[0]?.[0] as number | undefined;
    const secondCall = onchange.mock.calls[1]?.[0] as number | undefined;
    expect(firstCall).toBeCloseTo(0.9, 5);
    expect(secondCall).toBeCloseTo(0.2, 5);
  });

  it('pointermove without prior pointerdown is a no-op (drag gate)', async () => {
    const onchange = vi.fn();
    const { container } = render(Fader, { props: { value: 0.5, onchange } });
    const track = container.querySelector('.fader-track') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 100, 0);
    await fireEvent.pointerMove(track, { clientX: 5, clientY: 10, pointerId: 1 });
    expect(onchange).not.toHaveBeenCalled();
  });

  it('pointerdown above the track clamps onchange to 1 (top of range)', async () => {
    const onchange = vi.fn();
    const { container } = render(Fader, { props: { value: 0.5, onchange } });
    const track = container.querySelector('.fader-track') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 100, 0);
    stubPointerCapture(track);
    // clientY=-20 (above the track) → raw = 1.2 → clamped to 1.
    await fireEvent.pointerDown(track, { clientX: 5, clientY: -20, pointerId: 1 });
    expect(onchange).toHaveBeenCalledWith(1);
  });

  it('pointerdown below the track clamps onchange to 0 (bottom of range)', async () => {
    const onchange = vi.fn();
    const { container } = render(Fader, { props: { value: 0.5, onchange } });
    const track = container.querySelector('.fader-track') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 100, 0);
    stubPointerCapture(track);
    // clientY=200 (below the track) → raw = -1 → clamped to 0.
    await fireEvent.pointerDown(track, { clientX: 5, clientY: 200, pointerId: 1 });
    expect(onchange).toHaveBeenCalledWith(0);
  });

  it('disabled=true suppresses pointerdown onchange and adds disabled class', async () => {
    const onchange = vi.fn();
    const { container } = render(Fader, {
      props: { value: 0.5, onchange, disabled: true },
    });
    const root = container.querySelector('.fader');
    expect(root?.classList.contains('fader--disabled')).toBe(true);
    const track = container.querySelector('.fader-track') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    expect(track.getAttribute('aria-disabled')).toBe('true');
    stubTrackRect(track, 100, 0);
    stubPointerCapture(track);
    await fireEvent.pointerDown(track, { clientX: 5, clientY: 25, pointerId: 1 });
    expect(onchange).not.toHaveBeenCalled();
  });
});
