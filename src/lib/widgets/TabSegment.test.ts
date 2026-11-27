// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W40 TabSegment widget tests — verifies the prop → render contract for
// the segmented-button primitive consumed by upcoming Settings sub-section
// ports (Quantize / Color Space / Multi-Display).
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import TabSegment from './TabSegment.svelte';

const OPTIONS = [
  { id: 'off', label: 'off' },
  { id: 'beat', label: 'beat' },
  { id: 'bar', label: 'bar' },
  { id: 'phrase', label: 'phrase' },
] as const;

describe('TabSegment widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders one button per option', () => {
    const { getAllByRole } = render(TabSegment, { props: { options: OPTIONS, value: 'bar' } });
    expect(getAllByRole('tab')).toHaveLength(OPTIONS.length);
  });

  it('marks the active option with aria-selected and the active class', () => {
    const { getAllByRole } = render(TabSegment, { props: { options: OPTIONS, value: 'bar' } });
    const tabs = getAllByRole('tab');
    const bar = tabs[2];
    expect(bar).toBeDefined();
    expect(bar).toHaveAttribute('aria-selected', 'true');
    expect(bar?.classList.contains('active')).toBe(true);
  });

  it('leaves non-active options without the active class', () => {
    const { getAllByRole } = render(TabSegment, { props: { options: OPTIONS, value: 'bar' } });
    const tabs = getAllByRole('tab');
    expect(tabs[0]?.getAttribute('aria-selected')).toBe('false');
    expect(tabs[0]?.classList.contains('active')).toBe(false);
  });

  it('invokes onChange with the picked id when a non-active tab is clicked', () => {
    const onChange = vi.fn<(id: string) => void>();
    const { getAllByRole } = render(TabSegment, {
      props: { options: OPTIONS, value: 'bar', onChange },
    });
    const tabs = getAllByRole('tab');
    tabs[0]?.click();
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('off');
  });

  it('does not invoke onChange when the already-active tab is clicked', () => {
    const onChange = vi.fn<(id: string) => void>();
    const { getAllByRole } = render(TabSegment, {
      props: { options: OPTIONS, value: 'bar', onChange },
    });
    const tabs = getAllByRole('tab');
    tabs[2]?.click();
    expect(onChange).not.toHaveBeenCalled();
  });

  it('defaults the tablist aria-label to "Segmented options"', () => {
    const { getByRole } = render(TabSegment, { props: { options: OPTIONS, value: 'off' } });
    expect(getByRole('tablist')).toHaveAttribute('aria-label', 'Segmented options');
  });

  it('applies a custom ariaLabel to the tablist wrapper', () => {
    const { getByRole } = render(TabSegment, {
      props: { options: OPTIONS, value: 'off', ariaLabel: 'Quantize transitions' },
    });
    expect(getByRole('tablist')).toHaveAttribute('aria-label', 'Quantize transitions');
  });
});
