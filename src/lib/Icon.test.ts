// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W47 Icon shell-primitive tests — verifies the W19 PR3 ported icon
// dispatcher renders the right SVG for the requested name + size, and
// honours the locked design choices (24×24 viewBox, currentColor stroke,
// aria-hidden=true). `afterEach(cleanup)` per
// `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Icon from './Icon.svelte';

describe('Icon shell primitive', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders an svg with the default 16px size', () => {
    const { container } = render(Icon, { props: { name: 'play' } });
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg?.getAttribute('width')).toBe('16');
    expect(svg?.getAttribute('height')).toBe('16');
  });

  it('honours the size prop', () => {
    const { container } = render(Icon, { props: { name: 'play', size: 32 } });
    const svg = container.querySelector('svg');
    expect(svg?.getAttribute('width')).toBe('32');
    expect(svg?.getAttribute('height')).toBe('32');
  });

  it('uses the canonical 24×24 viewBox', () => {
    const { container } = render(Icon, { props: { name: 'play' } });
    expect(container.querySelector('svg')?.getAttribute('viewBox')).toBe('0 0 24 24');
  });

  it('inherits color via currentColor stroke', () => {
    const { container } = render(Icon, { props: { name: 'play' } });
    expect(container.querySelector('svg')?.getAttribute('stroke')).toBe('currentColor');
  });

  it('marks the icon aria-hidden by default (decorative)', () => {
    const { container } = render(Icon, { props: { name: 'play' } });
    expect(container.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders different shapes for different names', () => {
    const playRender = render(Icon, { props: { name: 'play' } });
    const pauseRender = render(Icon, { props: { name: 'pause' } });
    const playInner = playRender.container.querySelector('svg')?.innerHTML;
    const pauseInner = pauseRender.container.querySelector('svg')?.innerHTML;
    expect(playInner).not.toBe(pauseInner);
    expect(playInner?.length).toBeGreaterThan(0);
    expect(pauseInner?.length).toBeGreaterThan(0);
  });

  it('renders the perform icon as a rect + path stack', () => {
    const { container } = render(Icon, { props: { name: 'perform' } });
    const svg = container.querySelector('svg');
    expect(svg?.querySelectorAll('rect').length).toBe(1);
    expect(svg?.querySelectorAll('path').length).toBe(1);
  });

  it('renders the random icon with four corner dots (circles)', () => {
    const { container } = render(Icon, { props: { name: 'random' } });
    expect(container.querySelectorAll('svg circle').length).toBe(4);
  });
});
