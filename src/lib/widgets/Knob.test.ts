// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W37 Knob widget tests — extended in Wave 2.1 Lane A.2 to cover
// pointer-drag-scrub → `onchange` dispatch. The W8 baseline contract
// (-135° / 0° / +135° sweep, label / showValue render) is preserved;
// the W2.1 A.2 axis (Y-delta scrub interactivity) gates on the
// optional `onchange` callback presence (display-only callers stay
// non-interactive).
//
// Knob scrub maps Δy to Δvalue at 150 px per full sweep with Y
// inverted (drag-up increases value), clamped to [0, 1]. Pointer-event
// stubs mirror W46 / W37 conventions.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import Knob from './Knob.svelte';

function stubPointerCapture(target: HTMLElement): void {
  const captured = new Set<number>();
  target.setPointerCapture = (pointerId: number) => {
    captured.add(pointerId);
  };
  target.releasePointerCapture = (pointerId: number) => {
    captured.delete(pointerId);
  };
  target.hasPointerCapture = (pointerId: number) => captured.has(pointerId);
}

describe('Knob widget', () => {
  afterEach(() => {
    cleanup();
  });

  // ---------------- W8 display contract (preserved) ----------------------

  it('renders the dial + arc + indicator structure', () => {
    const { container } = render(Knob, { props: { value: 0.5 } });
    expect(container.querySelector('.knob')).not.toBeNull();
    expect(container.querySelector('.knob-dial')).not.toBeNull();
    expect(container.querySelector('.knob-arc')).not.toBeNull();
    expect(container.querySelector('.knob-ind')).not.toBeNull();
  });

  it('rotates -135deg at value=0 (sweep start)', () => {
    const { container } = render(Knob, { props: { value: 0 } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial?.getAttribute('style')).toContain('--rot: -135deg');
  });

  it('rotates 0deg at value=0.5 (sweep midpoint)', () => {
    const { container } = render(Knob, { props: { value: 0.5 } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial?.getAttribute('style')).toContain('--rot: 0deg');
  });

  it('rotates 135deg at value=1 (sweep end)', () => {
    const { container } = render(Knob, { props: { value: 1 } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial?.getAttribute('style')).toContain('--rot: 135deg');
  });

  it('clamps values above 1 to the sweep-end rotation', () => {
    const { container } = render(Knob, { props: { value: 5 } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial?.getAttribute('style')).toContain('--rot: 135deg');
  });

  it('renders a label when the prop is provided', () => {
    const { getByText } = render(Knob, { props: { value: 0.5, label: 'cutoff' } });
    expect(getByText('cutoff')).toBeInTheDocument();
  });

  it('omits the value readout by default', () => {
    const { container } = render(Knob, { props: { value: 0.42 } });
    expect(container.querySelector('.knob-val')).toBeNull();
  });

  it('renders the value readout to two decimal places when showValue=true', () => {
    const { getByText } = render(Knob, { props: { value: 0.42, showValue: true } });
    expect(getByText('0.42')).toBeInTheDocument();
  });

  // ---------------- Wave 2.1 Lane A.2 interactivity ---------------------

  it('stays role="presentation" when no onchange callback is provided', () => {
    const { container } = render(Knob, { props: { value: 0.5 } });
    const dial = container.querySelector('.knob-dial');
    expect(dial?.getAttribute('role')).toBe('presentation');
    expect(dial?.getAttribute('aria-valuenow')).toBeNull();
  });

  it('exposes role="slider" + aria-value* when onchange is provided', () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial');
    expect(dial?.getAttribute('role')).toBe('slider');
    expect(dial?.getAttribute('aria-valuemin')).toBe('0');
    expect(dial?.getAttribute('aria-valuemax')).toBe('1');
    expect(dial?.getAttribute('aria-valuenow')).toBe('0.5');
  });

  it('pointermove with upward Δy increases the value (scrub direction)', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial).not.toBeNull();
    if (dial === null) return;
    stubPointerCapture(dial);
    await fireEvent.pointerDown(dial, { clientX: 24, clientY: 100, pointerId: 1 });
    // Drag up 30 px → Δvalue = 30/150 = 0.2 → 0.5 + 0.2 = 0.7.
    await fireEvent.pointerMove(dial, { clientX: 24, clientY: 70, pointerId: 1 });
    expect(onchange).toHaveBeenCalledWith(0.7);
  });

  it('pointermove with downward Δy decreases the value', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial).not.toBeNull();
    if (dial === null) return;
    stubPointerCapture(dial);
    await fireEvent.pointerDown(dial, { clientX: 24, clientY: 100, pointerId: 1 });
    // Drag down 30 px → Δvalue = -0.2 → 0.5 - 0.2 = 0.3.
    await fireEvent.pointerMove(dial, { clientX: 24, clientY: 130, pointerId: 1 });
    expect(onchange).toHaveBeenCalledWith(0.3);
  });

  it('large upward drag clamps to 1', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial).not.toBeNull();
    if (dial === null) return;
    stubPointerCapture(dial);
    await fireEvent.pointerDown(dial, { clientX: 24, clientY: 200, pointerId: 1 });
    // Δy = -300 → Δvalue = +2 → clamped to 1.
    await fireEvent.pointerMove(dial, { clientX: 24, clientY: -100, pointerId: 1 });
    expect(onchange).toHaveBeenLastCalledWith(1);
  });

  it('large downward drag clamps to 0', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial).not.toBeNull();
    if (dial === null) return;
    stubPointerCapture(dial);
    await fireEvent.pointerDown(dial, { clientX: 24, clientY: 100, pointerId: 1 });
    // Δy = +300 → Δvalue = -2 → clamped to 0.
    await fireEvent.pointerMove(dial, { clientX: 24, clientY: 400, pointerId: 1 });
    expect(onchange).toHaveBeenLastCalledWith(0);
  });

  it('pointermove without pointerdown is a no-op (drag gate)', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial).not.toBeNull();
    if (dial === null) return;
    await fireEvent.pointerMove(dial, { clientX: 24, clientY: 70, pointerId: 1 });
    expect(onchange).not.toHaveBeenCalled();
  });

  it('disabled=true suppresses scrub and adds disabled class', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, {
      props: { value: 0.5, onchange, disabled: true },
    });
    const root = container.querySelector('.knob');
    expect(root?.classList.contains('knob--disabled')).toBe(true);
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    expect(dial).not.toBeNull();
    if (dial === null) return;
    expect(dial.getAttribute('aria-disabled')).toBe('true');
    stubPointerCapture(dial);
    await fireEvent.pointerDown(dial, { clientX: 24, clientY: 100, pointerId: 1 });
    await fireEvent.pointerMove(dial, { clientX: 24, clientY: 70, pointerId: 1 });
    expect(onchange).not.toHaveBeenCalled();
  });

  // ---------------- DESIGN.md "Bundle alignment" — keyboard nudge ---------

  it('ArrowUp nudges value up by 5% step', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    if (dial === null) throw new Error('knob-dial not rendered');
    await fireEvent.keyDown(dial, { key: 'ArrowUp' });
    expect(onchange).toHaveBeenCalledWith(0.55);
  });

  it('ArrowLeft nudges value down by 5% step', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    if (dial === null) throw new Error('knob-dial not rendered');
    await fireEvent.keyDown(dial, { key: 'ArrowLeft' });
    const got = onchange.mock.calls[0]?.[0] as number | undefined;
    expect(got).toBeCloseTo(0.45, 5);
  });

  it('Shift+ArrowUp uses the 20% large step', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.3, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    if (dial === null) throw new Error('knob-dial not rendered');
    await fireEvent.keyDown(dial, { key: 'ArrowUp', shiftKey: true });
    const got = onchange.mock.calls[0]?.[0] as number | undefined;
    expect(got).toBeCloseTo(0.5, 5);
  });

  it('Home / End snap to extremes', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, { props: { value: 0.5, onchange } });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    if (dial === null) throw new Error('knob-dial not rendered');
    await fireEvent.keyDown(dial, { key: 'End' });
    expect(onchange).toHaveBeenLastCalledWith(1);
    await fireEvent.keyDown(dial, { key: 'Home' });
    expect(onchange).toHaveBeenLastCalledWith(0);
  });

  it('disabled knob does not fire onchange on keydown', async () => {
    const onchange = vi.fn();
    const { container } = render(Knob, {
      props: { value: 0.5, onchange, disabled: true },
    });
    const dial = container.querySelector('.knob-dial') as HTMLElement | null;
    if (dial === null) throw new Error('knob-dial not rendered');
    await fireEvent.keyDown(dial, { key: 'ArrowUp' });
    expect(onchange).not.toHaveBeenCalled();
  });
});
