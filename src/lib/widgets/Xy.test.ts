// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W32 Xy primitive tests — exercises the normalised dot positioning
// against [-1, 1] and [0, 1] ranges, plus the readout formatting.
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Xy from './Xy.svelte';

describe('Xy widget', () => {
  afterEach(() => {
    cleanup();
  });

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

  it('respects a [0, 1] explicit range', () => {
    const { container } = render(Xy, { props: { value: [0.5, 0.5], min: [0, 0], max: [1, 1] } });
    const dot = container.querySelector('.xy__dot') as HTMLElement | null;
    expect(dot).not.toBeNull();
    if (dot === null) return;
    expect(dot.style.left).toBe('50%');
    expect(dot.style.top).toBe('50%');
  });
});
