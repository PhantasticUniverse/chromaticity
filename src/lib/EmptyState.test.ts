// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W47 EmptyState shell tests — verifies the W8 empty-state surface
// renders the title, optional body, optional inline icon SVG, and the
// `actions` snippet slot. `afterEach(cleanup)` per
// `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import EmptyState from './EmptyState.svelte';

describe('EmptyState shell primitive', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the title heading', () => {
    const { container } = render(EmptyState, { props: { title: 'Nothing to show' } });
    const h2 = container.querySelector('.empty-state-title');
    expect(h2?.textContent).toBe('Nothing to show');
  });

  it('renders the body paragraph when provided', () => {
    const { container } = render(EmptyState, {
      props: { title: 'No scenes', body: 'Import a scene to begin.' },
    });
    expect(container.querySelector('.empty-state-body')?.textContent).toBe(
      'Import a scene to begin.',
    );
  });

  it('omits the body paragraph when no body prop is given', () => {
    const { container } = render(EmptyState, { props: { title: 'No scenes' } });
    expect(container.querySelector('.empty-state-body')).toBeNull();
  });

  it('renders the orbit-ring backdrop SVG with the canonical 160 viewBox', () => {
    const { container } = render(EmptyState, { props: { title: 'No scenes' } });
    const orbit = container.querySelector('.empty-orbit');
    expect(orbit).not.toBeNull();
    expect(orbit?.getAttribute('viewBox')).toBe('0 0 160 160');
    expect(orbit?.querySelectorAll('circle').length).toBe(2);
  });

  it('renders the inline icon SVG when iconName is provided', () => {
    const { container } = render(EmptyState, {
      props: { title: 'No scenes', iconName: 'library' },
    });
    const iconSvg = container.querySelector('.empty-state-icon');
    expect(iconSvg).not.toBeNull();
    expect(iconSvg?.getAttribute('data-icon')).toBe('library');
  });

  it('omits the inline icon SVG when no iconName is given', () => {
    const { container } = render(EmptyState, { props: { title: 'No scenes' } });
    expect(container.querySelector('.empty-state-icon')).toBeNull();
  });

  it('renders the actions snippet slot when supplied', () => {
    const actions = createRawSnippet(() => ({
      render: () => '<button type="button" data-testid="cta">Import</button>',
    }));
    const { container, getByTestId } = render(EmptyState, {
      props: { title: 'No scenes', actions },
    });
    expect(container.querySelector('.empty-state-actions')).not.toBeNull();
    expect(getByTestId('cta')).toBeInTheDocument();
  });

  it('omits the actions wrapper when no snippet is supplied', () => {
    const { container } = render(EmptyState, { props: { title: 'No scenes' } });
    expect(container.querySelector('.empty-state-actions')).toBeNull();
  });

  it('marks the wrapper with role="presentation"', () => {
    const { container } = render(EmptyState, { props: { title: 'No scenes' } });
    expect(container.querySelector('.empty-state')?.getAttribute('role')).toBe('presentation');
  });
});
