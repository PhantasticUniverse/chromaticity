// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Shared-primitive backfill — `RailIcon` widget tests. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
// lines 68-78. Covers the structural mount, label round-trip,
// active / aria-pressed branch, click handler dispatch, and the
// `aria-pressed` elision when inactive (per the bundle's
// `aria-pressed={active || undefined}` idiom).
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import RailIcon from './RailIcon.svelte';

describe('RailIcon widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders a button with the rail-btn class', () => {
    const { container } = render(RailIcon, {
      props: { name: 'list', label: 'Scenes' },
    });
    const btn = container.querySelector('button.rail-btn');
    expect(btn).not.toBeNull();
  });

  it('binds the label to both title and aria-label', () => {
    const { container } = render(RailIcon, {
      props: { name: 'grid', label: 'Presets' },
    });
    const btn = container.querySelector('button.rail-btn');
    expect(btn?.getAttribute('title')).toBe('Presets');
    expect(btn?.getAttribute('aria-label')).toBe('Presets');
  });

  it('omits aria-pressed when inactive (bundle parity)', () => {
    const { container } = render(RailIcon, {
      props: { name: 'edit', label: 'Edit' },
    });
    const btn = container.querySelector('button.rail-btn');
    expect(btn?.hasAttribute('aria-pressed')).toBe(false);
    expect(btn?.classList.contains('active')).toBe(false);
  });

  it('sets aria-pressed=true and the active class when active', () => {
    const { container } = render(RailIcon, {
      props: { name: 'plus', label: 'Add', active: true },
    });
    const btn = container.querySelector('button.rail-btn');
    expect(btn?.getAttribute('aria-pressed')).toBe('true');
    expect(btn?.classList.contains('active')).toBe(true);
  });

  it('invokes onClick when clicked', async () => {
    const onClick = vi.fn();
    const { container } = render(RailIcon, {
      props: { name: 'list', label: 'Scenes', onClick },
    });
    const btn = container.querySelector('button.rail-btn');
    expect(btn).not.toBeNull();
    await fireEvent.click(btn as Element);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('renders the requested icon at the given size', () => {
    const { container } = render(RailIcon, {
      props: { name: 'play', label: 'Play', size: 20 },
    });
    const svg = container.querySelector('button svg');
    expect(svg).not.toBeNull();
    expect(svg?.getAttribute('width')).toBe('20');
    expect(svg?.getAttribute('height')).toBe('20');
  });

  it('declares type=button so it does not submit a parent form', () => {
    const { container } = render(RailIcon, {
      props: { name: 'list', label: 'Scenes' },
    });
    const btn = container.querySelector('button.rail-btn');
    expect(btn?.getAttribute('type')).toBe('button');
  });
});
