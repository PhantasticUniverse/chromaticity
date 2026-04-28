// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W37 Fader widget tests — close the W8 core-primitive coverage gap
// flagged in the 2026-04-28 widget-coverage audit. The W8 visual
// primitive is display-only (no pointer interaction; W9 wires that),
// so the tests focus on the prop → render contract: value-to-fill
// percentage clamping, label conditional render, and the modulation-
// halo class toggle.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Fader from './Fader.svelte';

describe('Fader widget', () => {
  afterEach(() => {
    cleanup();
  });

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
});
