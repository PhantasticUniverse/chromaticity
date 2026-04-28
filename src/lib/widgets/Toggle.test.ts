// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W37 Toggle widget tests — close the W8 core-primitive coverage gap
// flagged in the 2026-04-28 widget-coverage audit. The W8 visual
// primitive is display-only (W9 wires click / IPC), so tests verify
// the prop → render contract: the `on` class reflects boolean value,
// orientation switches between vertical default and horizontal pill,
// and label is conditionally rendered.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Toggle from './Toggle.svelte';

describe('Toggle widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the switch structure', () => {
    const { container } = render(Toggle, { props: { value: false } });
    expect(container.querySelector('.tog')).not.toBeNull();
    expect(container.querySelector('.tog-switch')).not.toBeNull();
  });

  it('applies the `on` class when value=true', () => {
    const { container } = render(Toggle, { props: { value: true } });
    const root = container.querySelector('.tog');
    expect(root?.classList.contains('on')).toBe(true);
  });

  it('omits the `on` class when value=false', () => {
    const { container } = render(Toggle, { props: { value: false } });
    const root = container.querySelector('.tog');
    expect(root?.classList.contains('on')).toBe(false);
  });

  it('defaults to vertical orientation (no `tog-h` class)', () => {
    const { container } = render(Toggle, { props: { value: false } });
    const root = container.querySelector('.tog');
    expect(root?.classList.contains('tog-h')).toBe(false);
  });

  it('applies the `tog-h` class when orientation=horizontal', () => {
    const { container } = render(Toggle, { props: { value: false, orientation: 'horizontal' } });
    const root = container.querySelector('.tog');
    expect(root?.classList.contains('tog-h')).toBe(true);
  });

  it('renders a label when the prop is provided', () => {
    const { getByText } = render(Toggle, { props: { value: false, label: 'mute' } });
    expect(getByText('mute')).toBeInTheDocument();
  });

  it('omits the label DOM when no label prop is given', () => {
    const { container } = render(Toggle, { props: { value: false } });
    expect(container.querySelector('.tog-label')).toBeNull();
  });
});
