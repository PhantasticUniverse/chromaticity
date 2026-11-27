// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W46 CrossFader interaction tests — horizontal A↔B crossfader with
// pointer-drag. jsdom returns a 0×0 bounding rect by default; the
// pointermove math depends on `rect.width`, so the rect is stubbed on the
// track element before firing events to make the value math deterministic.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import CrossFader from './CrossFader.svelte';

function stubTrackRect(track: HTMLElement, width = 200, left = 0): void {
  track.getBoundingClientRect = (): DOMRect =>
    ({
      x: left,
      y: 0,
      left,
      top: 0,
      right: left + width,
      bottom: 20,
      width,
      height: 20,
      toJSON: () => ({}),
    }) as DOMRect;
}

// jsdom does not implement `setPointerCapture` / `releasePointerCapture`
// on HTMLElement; stub them so the pointer handlers run to completion.
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

describe('CrossFader widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the wrap + track + thumb structure with default A/B labels', () => {
    const { container } = render(CrossFader);
    expect(container.querySelector('.xfade-wrap')).not.toBeNull();
    expect(container.querySelector('.xfade')).not.toBeNull();
    expect(container.querySelector('.xfade-thumb')).not.toBeNull();
    const labels = container.querySelectorAll('.xfade-labels span');
    expect(labels[0]?.textContent).toBe('A');
    expect(labels[1]?.textContent).toBe('B');
  });

  it('reflects value=0.5 as a 50% --xfade-pos custom property', () => {
    const { container } = render(CrossFader, { props: { value: 0.5 } });
    const wrap = container.querySelector('.xfade-wrap') as HTMLElement | null;
    expect(wrap?.getAttribute('style')).toContain('--xfade-pos: 50%');
  });

  it('clamps value above 1 to 100% and below 0 to 0% in --xfade-pos', () => {
    const { container, rerender } = render(CrossFader, { props: { value: 1.5 } });
    expect(
      (container.querySelector('.xfade-wrap') as HTMLElement | null)?.getAttribute('style'),
    ).toContain('--xfade-pos: 100%');
    rerender({ value: -0.4 });
    expect(
      (container.querySelector('.xfade-wrap') as HTMLElement | null)?.getAttribute('style'),
    ).toContain('--xfade-pos: 0%');
  });

  it('exposes a slider role with aria-valuemin/max/now matching value', () => {
    const { container } = render(CrossFader, { props: { value: 0.5 } });
    const slider = container.querySelector('[role="slider"]');
    expect(slider).not.toBeNull();
    expect(slider?.getAttribute('aria-valuemin')).toBe('0');
    expect(slider?.getAttribute('aria-valuemax')).toBe('1');
    expect(slider?.getAttribute('aria-valuenow')).toBe('0.5');
    expect(slider?.getAttribute('aria-label')).toBe('Crossfader A to B');
  });

  it('pointerdown on the track fires onChange with the clientX-derived value', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, {
      props: { value: 0.5, onChange },
    });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 200, 0);
    stubPointerCapture(track);
    // clientX=150, rect.left=0, width=200 → raw = 150/200 = 0.75.
    await fireEvent.pointerDown(track, { clientX: 150, clientY: 10, pointerId: 1 });
    expect(onChange).toHaveBeenCalledWith(0.75);
  });

  it('pointermove after pointerdown fires onChange with updated values', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, {
      props: { value: 0.5, onChange },
    });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 200, 0);
    stubPointerCapture(track);
    await fireEvent.pointerDown(track, { clientX: 100, clientY: 10, pointerId: 1 });
    onChange.mockClear();
    await fireEvent.pointerMove(track, { clientX: 50, clientY: 10, pointerId: 1 });
    await fireEvent.pointerMove(track, { clientX: 180, clientY: 10, pointerId: 1 });
    expect(onChange).toHaveBeenNthCalledWith(1, 0.25);
    expect(onChange).toHaveBeenNthCalledWith(2, 0.9);
  });

  it('pointermove without prior pointerdown is a no-op (drag gate)', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, {
      props: { value: 0.5, onChange },
    });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    stubTrackRect(track, 200, 0);
    await fireEvent.pointerMove(track, { clientX: 100, clientY: 10, pointerId: 1 });
    expect(onChange).not.toHaveBeenCalled();
  });

  it('disabled=true suppresses pointerdown onChange and sets aria-disabled', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, {
      props: { value: 0.5, onChange, disabled: true },
    });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    expect(track).not.toBeNull();
    if (track === null) return;
    expect(track.getAttribute('aria-disabled')).toBe('true');
    expect(track.getAttribute('tabindex')).toBe('-1');
    stubTrackRect(track, 200, 0);
    stubPointerCapture(track);
    await fireEvent.pointerDown(track, { clientX: 150, clientY: 10, pointerId: 1 });
    expect(onChange).not.toHaveBeenCalled();
    expect(container.querySelector('.xfade-wrap--disabled')).not.toBeNull();
  });

  // ---------------- DESIGN.md "Bundle alignment" — keyboard nudge ---------

  it('ArrowRight nudges value up by 5% step', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, { props: { value: 0.5, onChange } });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    if (track === null) throw new Error('xfade not rendered');
    await fireEvent.keyDown(track, { key: 'ArrowRight' });
    expect(onChange).toHaveBeenCalledWith(0.55);
  });

  it('ArrowLeft nudges value down by 5% step', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, { props: { value: 0.5, onChange } });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    if (track === null) throw new Error('xfade not rendered');
    await fireEvent.keyDown(track, { key: 'ArrowLeft' });
    const got = onChange.mock.calls[0]?.[0] as number | undefined;
    expect(got).toBeCloseTo(0.45, 5);
  });

  it('Home / End snap to extremes', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, { props: { value: 0.5, onChange } });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    if (track === null) throw new Error('xfade not rendered');
    await fireEvent.keyDown(track, { key: 'End' });
    expect(onChange).toHaveBeenLastCalledWith(1);
    await fireEvent.keyDown(track, { key: 'Home' });
    expect(onChange).toHaveBeenLastCalledWith(0);
  });

  it('disabled crossfader does not fire onChange on keydown', async () => {
    const onChange = vi.fn();
    const { container } = render(CrossFader, {
      props: { value: 0.5, onChange, disabled: true },
    });
    const track = container.querySelector('.xfade') as HTMLElement | null;
    if (track === null) throw new Error('xfade not rendered');
    await fireEvent.keyDown(track, { key: 'ArrowRight' });
    expect(onChange).not.toHaveBeenCalled();
  });
});
