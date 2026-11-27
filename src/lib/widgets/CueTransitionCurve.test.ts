// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-13 PR2 spec for `CueTransitionCurve.svelte`. Exercises the pure
// math module (`bezier1d`, `presetFor`) and the component's render +
// interaction surfaces. Vitest globals are disabled in
// `vitest.config.ts`; describe / it / expect / vi are imported
// explicitly per the W11 PR3 baseline (`ErrorMessage.test.ts`).

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { bezier1d, CUE_PRESETS, presetFor } from './cue-transition-curve.math';
import CueTransitionCurve from './CueTransitionCurve.svelte';

// `globals: false` in vitest.config.ts means @testing-library/svelte's
// auto-cleanup `beforeEach` / `afterEach` registration is skipped. Wire
// it explicitly so each test starts with a fresh DOM.
afterEach(() => {
  cleanup();
});

describe('cue-transition-curve.math', () => {
  describe('bezier1d', () => {
    it('returns p0 at t=0', () => {
      expect(bezier1d(0, 0, 0.42, 0.58, 1)).toBeCloseTo(0, 10);
    });

    it('returns p3 at t=1', () => {
      expect(bezier1d(1, 0, 0.42, 0.58, 1)).toBeCloseTo(1, 10);
    });

    it('LINEAR (cp1=0, cp2=1) passes through 0.5 at t=0.5', () => {
      expect(bezier1d(0.5, 0, 0, 1, 1)).toBeCloseTo(0.5, 10);
    });

    it('matches the closed-form expansion at t=0.25', () => {
      // B(t) = (1-t)^3·p0 + 3(1-t)^2·t·p1 + 3(1-t)·t^2·p2 + t^3·p3.
      // For t=0.25, p0=0, p1=0.42, p2=0.58, p3=1:
      const t = 0.25;
      const expected =
        Math.pow(1 - t, 3) * 0 +
        3 * Math.pow(1 - t, 2) * t * 0.42 +
        3 * (1 - t) * Math.pow(t, 2) * 0.58 +
        Math.pow(t, 3) * 1;
      expect(bezier1d(t, 0, 0.42, 0.58, 1)).toBeCloseTo(expected, 10);
    });
  });

  describe('presetFor', () => {
    it('matches the LINEAR preset for cp1=(0,0), cp2=(1,1)', () => {
      const found = presetFor([0, 0], [1, 1]);
      expect(found?.id).toBe('linear');
    });

    it('matches the EASE-IN preset for cp1=(0.42,0), cp2=(1,1)', () => {
      const found = presetFor([0.42, 0], [1, 1]);
      expect(found?.id).toBe('in');
    });

    it('returns undefined for arbitrary non-preset control points', () => {
      const found = presetFor([0.31, 0.17], [0.62, 0.83]);
      expect(found).toBeUndefined();
    });

    it('rounds to two-decimal precision when matching presets', () => {
      // 0.42001 rounds to 0.42 and should still match the EASE-IN preset.
      const found = presetFor([0.42001, 0], [1, 1]);
      expect(found?.id).toBe('in');
    });

    it('exposes a non-empty preset catalogue', () => {
      expect(CUE_PRESETS.length).toBeGreaterThanOrEqual(6);
    });
  });
});

describe('CueTransitionCurve', () => {
  it('renders default props with FADE active and a bezier path', () => {
    const { container } = render(CueTransitionCurve);
    const root = container.querySelector('.cue-curve');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('data-active-kind')).toBe('fade');
    expect(container.querySelector('path.cue-curve__path')).not.toBeNull();
    // FADE mode shows a ghost dot.
    expect(container.querySelector('circle.cue-curve__ghost')).not.toBeNull();
  });

  it('switches to CUT when the Cut tab is clicked and emits onchange', async () => {
    const onchange = vi.fn();
    const { container, getByRole } = render(CueTransitionCurve, {
      props: { onchange },
    });
    const cutTab = getByRole('tab', { name: 'Cut' });
    await fireEvent.click(cutTab);
    expect(container.querySelector('.cue-curve')?.getAttribute('data-active-kind')).toBe('cut');
    expect(onchange).toHaveBeenCalled();
    const lastCall = onchange.mock.calls.at(-1)?.[0];
    expect(lastCall).toMatchObject({ kind: 'cut' });
  });

  it('applying a preset updates the active-preset marker', async () => {
    const { container, getByRole } = render(CueTransitionCurve, {
      props: { cp1: [0.31, 0.17], cp2: [0.62, 0.83] },
    });
    // Custom (no match) is the initial state.
    expect(container.querySelector('.cue-curve__presets')?.getAttribute('data-active-preset')).toBe(
      'custom',
    );
    const linearBtn = getByRole('tab', { name: 'Linear' });
    await fireEvent.click(linearBtn);
    expect(container.querySelector('.cue-curve__presets')?.getAttribute('data-active-preset')).toBe(
      'linear',
    );
  });

  it('snap-mode pill click emits onchange with the new snap divisor', async () => {
    const onchange = vi.fn();
    const { getByRole } = render(CueTransitionCurve, {
      props: { kind: 'snap', onchange },
    });
    const half = getByRole('tab', { name: /1\/2/ });
    await fireEvent.click(half);
    expect(onchange).toHaveBeenCalled();
    const lastCall = onchange.mock.calls.at(-1)?.[0];
    expect(lastCall).toMatchObject({ kind: 'snap', snap: '0.5' });
  });

  it('re-syncs cp1 / cp2 when props change', async () => {
    // W14 PR1 regression guard: the W13 PR2 implementation seeded local
    // `$state` from props via `untrack(...)` AND lacked the prop-change
    // re-sync `$effect`, so calling `rerender({...})` left the curve
    // visual stuck at first-mount values. With the fix, prop changes
    // propagate to the SVG `path` `d` attribute on the next render.
    const { container, rerender } = render(CueTransitionCurve, {
      props: {
        kind: 'fade' as const,
        cp1: [0.42, 0.0] as [number, number],
        cp2: [0.58, 1.0] as [number, number],
      },
    });
    const initialPath = container.querySelector('path.cue-curve__path')?.getAttribute('d');
    expect(initialPath).not.toBeNull();
    expect(initialPath).toContain('C');

    await rerender({
      kind: 'fade' as const,
      cp1: [0.7, 0.0] as [number, number],
      cp2: [0.3, 1.0] as [number, number],
    });
    const updatedPath = container.querySelector('path.cue-curve__path')?.getAttribute('d');
    expect(updatedPath).not.toBeNull();
    expect(updatedPath).not.toBe(initialPath);

    // Numeric readout reflects the new control points (rendered to two
    // decimal places via `toFixed(2)`).
    const readout = container.querySelector('.cue-curve__readout')?.textContent ?? '';
    expect(readout).toContain('0.70');
    expect(readout).toContain('0.30');
  });

  it('re-syncs kind when props change', async () => {
    // Kind prop change must flip the active mode tab. Pre-fix the local
    // `activeKind` was snapshotted at mount and did not update.
    const { container, rerender, getByRole } = render(CueTransitionCurve, {
      props: { kind: 'fade' as const },
    });
    expect(container.querySelector('.cue-curve')?.getAttribute('data-active-kind')).toBe('fade');
    expect(getByRole('tab', { name: 'Fade' }).getAttribute('aria-selected')).toBe('true');

    await rerender({ kind: 'cut' as const });
    expect(container.querySelector('.cue-curve')?.getAttribute('data-active-kind')).toBe('cut');
    expect(getByRole('tab', { name: 'Cut' }).getAttribute('aria-selected')).toBe('true');
  });

  it('pointerdown on a handle followed by window pointermove fires onchange with new cp values', async () => {
    const onchange = vi.fn();
    const { container } = render(CueTransitionCurve, {
      props: { onchange },
    });
    const cp1Handle = container.querySelector('[data-handle="cp1"]');
    expect(cp1Handle).not.toBeNull();
    if (cp1Handle === null) return;

    // Fake the SVG bounding rect — jsdom returns zeros otherwise.
    const svg = container.querySelector('svg.cue-curve__svg');
    expect(svg).not.toBeNull();
    if (svg === null) return;
    svg.getBoundingClientRect = (): DOMRect =>
      ({
        x: 0,
        y: 0,
        left: 0,
        top: 0,
        right: 280,
        bottom: 148,
        width: 280,
        height: 148,
        toJSON: () => ({}),
      }) as DOMRect;

    onchange.mockClear();
    await fireEvent.pointerDown(cp1Handle, { clientX: 0, clientY: 0 });
    // Inside the SVG: PAD_X=16, PAD_Y=14, IW=248, IH=120. Pointer at
    // (clientX=78, clientY=74) → x = (78-16)/248 = 0.25, y = 1 - (74-14)/120 = 0.5
    await fireEvent.pointerMove(window, { clientX: 78, clientY: 74 });
    await fireEvent.pointerUp(window);
    expect(onchange).toHaveBeenCalled();
    const lastCall = onchange.mock.calls.at(-1)?.[0];
    expect(lastCall.cp1[0]).toBeCloseTo(0.25, 5);
    expect(lastCall.cp1[1]).toBeCloseTo(0.5, 5);
  });

  // ---------------- DESIGN.md "Bundle alignment" — 2D keyboard nudge -------

  it('ArrowRight on cp1 nudges X by 5%', async () => {
    const onchange = vi.fn();
    const { container } = render(CueTransitionCurve, {
      props: { kind: 'fade' as const, cp1: [0.42, 0.0] as [number, number], onchange },
    });
    const cp1 = container.querySelector('[data-handle="cp1"]') as SVGGElement | null;
    if (cp1 === null) throw new Error('cp1 handle not rendered');
    await fireEvent.keyDown(cp1, { key: 'ArrowRight' });
    expect(onchange).toHaveBeenCalled();
    const got = onchange.mock.calls.at(-1)?.[0];
    expect(got.cp1[0]).toBeCloseTo(0.47, 5);
    expect(got.cp1[1]).toBeCloseTo(0.0, 5);
  });

  it('ArrowUp on cp2 nudges Y by 5%', async () => {
    const onchange = vi.fn();
    const { container } = render(CueTransitionCurve, {
      props: { kind: 'fade' as const, cp2: [0.58, 0.5] as [number, number], onchange },
    });
    const cp2 = container.querySelector('[data-handle="cp2"]') as SVGGElement | null;
    if (cp2 === null) throw new Error('cp2 handle not rendered');
    await fireEvent.keyDown(cp2, { key: 'ArrowUp' });
    const got = onchange.mock.calls.at(-1)?.[0];
    expect(got.cp2[0]).toBeCloseTo(0.58, 5);
    expect(got.cp2[1]).toBeCloseTo(0.55, 5);
  });

  it('Shift+ArrowRight uses the 20% large X step', async () => {
    const onchange = vi.fn();
    const { container } = render(CueTransitionCurve, {
      props: { kind: 'fade' as const, cp1: [0.2, 0.0] as [number, number], onchange },
    });
    const cp1 = container.querySelector('[data-handle="cp1"]') as SVGGElement | null;
    if (cp1 === null) throw new Error('cp1 handle not rendered');
    await fireEvent.keyDown(cp1, { key: 'ArrowRight', shiftKey: true });
    const got = onchange.mock.calls.at(-1)?.[0];
    expect(got.cp1[0]).toBeCloseTo(0.4, 5);
  });

  it('Home snaps X to 0', async () => {
    const onchange = vi.fn();
    const { container } = render(CueTransitionCurve, {
      props: { kind: 'fade' as const, cp1: [0.5, 0.3] as [number, number], onchange },
    });
    const cp1 = container.querySelector('[data-handle="cp1"]') as SVGGElement | null;
    if (cp1 === null) throw new Error('cp1 handle not rendered');
    await fireEvent.keyDown(cp1, { key: 'Home' });
    const got = onchange.mock.calls.at(-1)?.[0];
    expect(got.cp1[0]).toBe(0);
    expect(got.cp1[1]).toBeCloseTo(0.3, 5);
  });

  it('clamps to [0, 1] at the extremes', async () => {
    const onchange = vi.fn();
    const { container } = render(CueTransitionCurve, {
      props: { kind: 'fade' as const, cp2: [0.98, 0.5] as [number, number], onchange },
    });
    const cp2 = container.querySelector('[data-handle="cp2"]') as SVGGElement | null;
    if (cp2 === null) throw new Error('cp2 handle not rendered');
    await fireEvent.keyDown(cp2, { key: 'ArrowRight' });
    const got = onchange.mock.calls.at(-1)?.[0];
    expect(got.cp2[0]).toBe(1);
  });
});
