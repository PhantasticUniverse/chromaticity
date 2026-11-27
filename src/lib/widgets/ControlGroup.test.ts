// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W37 ControlGroup widget tests — close the W8 core-primitive coverage
// gap flagged in the 2026-04-28 widget-coverage audit. The W8 visual
// primitive is a snippet-children container with name-keyed tint
// resolution. Tests cover the tint-resolution chain (explicit prop ↦
// name-keyed lookup ↦ general fallback), the children snippet render,
// and case-insensitive name matching.
//
// `createRawSnippet` from `svelte` is used to fabricate a minimal
// children snippet for the render contract; the wrapper component
// pattern (a separate `.svelte` test fixture) is overkill for the
// single-element test bodies here.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import ControlGroup from './ControlGroup.svelte';

const makeChildren = (text: string) =>
  createRawSnippet(() => ({
    render: () => `<div data-testid="snippet-child">${text}</div>`,
  }));

describe('ControlGroup widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the group structure with name in the head', () => {
    const { container, getByText } = render(ControlGroup, {
      props: { name: 'color', children: makeChildren('child') },
    });
    expect(container.querySelector('.group')).not.toBeNull();
    expect(container.querySelector('.group-head')).not.toBeNull();
    expect(container.querySelector('.group-grid')).not.toBeNull();
    expect(getByText('color')).toBeInTheDocument();
  });

  it('renders the children snippet content inside the grid', () => {
    const { getByTestId } = render(ControlGroup, {
      props: { name: 'color', children: makeChildren('inner content') },
    });
    expect(getByTestId('snippet-child').textContent).toBe('inner content');
  });

  it('uses the explicit tint prop when provided, overriding name lookup', () => {
    const { container } = render(ControlGroup, {
      props: { name: 'color', tint: 'rgb(1, 2, 3)', children: makeChildren('x') },
    });
    const root = container.querySelector('.group') as HTMLElement | null;
    expect(root?.getAttribute('style')).toContain('--tint: rgb(1, 2, 3)');
  });

  it('uses the name-keyed canonical tint when name matches', () => {
    const { container } = render(ControlGroup, {
      props: { name: 'color', children: makeChildren('x') },
    });
    const root = container.querySelector('.group') as HTMLElement | null;
    expect(root?.getAttribute('style')).toContain('--tint: var(--color-cat-color)');
  });

  it('falls back to general tint for unknown names', () => {
    const { container } = render(ControlGroup, {
      props: { name: 'not-canonical', children: makeChildren('x') },
    });
    const root = container.querySelector('.group') as HTMLElement | null;
    expect(root?.getAttribute('style')).toContain('--tint: var(--color-cat-general)');
  });

  it('matches canonical names case-insensitively', () => {
    const { container } = render(ControlGroup, {
      props: { name: 'COLOR', children: makeChildren('x') },
    });
    const root = container.querySelector('.group') as HTMLElement | null;
    expect(root?.getAttribute('style')).toContain('--tint: var(--color-cat-color)');
  });

  it('resolves all canonical category names', () => {
    const canonicalNames = ['color', 'motion', 'fx', 'media', 'limit', 'geom'] as const;
    for (const name of canonicalNames) {
      const { container, unmount } = render(ControlGroup, {
        props: { name, children: makeChildren('x') },
      });
      const root = container.querySelector('.group') as HTMLElement | null;
      expect(root?.getAttribute('style')).toContain(`--tint: var(--color-cat-${name})`);
      unmount();
    }
  });
});
