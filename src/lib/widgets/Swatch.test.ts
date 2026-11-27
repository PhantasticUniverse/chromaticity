// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W37 Swatch widget tests — close the W8 core-primitive coverage gap
// flagged in the 2026-04-28 widget-coverage audit. The W8 visual
// primitive is display-only (no picker UI), so tests verify the prop
// → render contract: the `color` prop passes through to the inline
// `background:` style verbatim, and the label renders conditionally.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Swatch from './Swatch.svelte';

describe('Swatch widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the wrap + swatch structure', () => {
    const { container } = render(Swatch, { props: { color: '#ff0000' } });
    expect(container.querySelector('.swatch-wrap')).not.toBeNull();
    expect(container.querySelector('.swatch')).not.toBeNull();
  });

  it('passes a literal hex color through to the inline background', () => {
    const { container } = render(Swatch, { props: { color: '#ff0000' } });
    const swatch = container.querySelector('.swatch') as HTMLElement | null;
    // jsdom normalises literal hex to rgb() in `getAttribute('style')`
    // even before computed-style resolution; assert against the
    // normalised form rather than the source literal.
    expect(swatch?.getAttribute('style')).toContain('background: rgb(255, 0, 0)');
  });

  it('passes a CSS variable color through verbatim', () => {
    const { container } = render(Swatch, {
      props: { color: 'var(--color-primary)' },
    });
    const swatch = container.querySelector('.swatch') as HTMLElement | null;
    expect(swatch?.getAttribute('style')).toContain('background: var(--color-primary)');
  });

  it('renders a label when the prop is provided', () => {
    const { getByText } = render(Swatch, { props: { color: 'red', label: 'accent' } });
    expect(getByText('accent')).toBeInTheDocument();
  });

  it('omits the label DOM when no label prop is given', () => {
    const { container } = render(Swatch, { props: { color: 'red' } });
    expect(container.querySelector('.swatch-label')).toBeNull();
  });
});
