// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W37 Knob widget tests — close the W8 core-primitive coverage gap
// flagged in the 2026-04-28 widget-coverage audit. The W8 visual
// primitive is display-only (no scrub / no detents; W9 wires pointer
// interaction), so the tests focus on the prop → render contract:
// the -135° / 0° / +135° rotation sweep at value 0 / 0.5 / 1, label
// conditional render, and the showValue numeric readout.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Knob from './Knob.svelte';

describe('Knob widget', () => {
  afterEach(() => {
    cleanup();
  });

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
});
