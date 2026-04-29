// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W48 ThemeSwitcher shell tests — verifies the W6 three-theme cycle
// widget renders the canonical pulse/atelier/console options, and that
// the $effect propagates the active theme to `document.documentElement`
// via the `data-theme` attribute (with `pulse` clearing the attribute
// per the component's stated invariant). `afterEach(cleanup)` per
// `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import ThemeSwitcher from './ThemeSwitcher.svelte';

describe('ThemeSwitcher shell component', () => {
  afterEach(() => {
    cleanup();
    document.documentElement.dataset.theme = '';
  });

  it('renders the theme label and a select element', () => {
    const { container, getByText } = render(ThemeSwitcher);
    expect(getByText('theme')).toBeInTheDocument();
    expect(container.querySelector('select')).not.toBeNull();
  });

  it('exposes the three canonical theme options in order', () => {
    const { container } = render(ThemeSwitcher);
    const options = Array.from(container.querySelectorAll('option')).map((o) => o.value);
    expect(options).toEqual(['pulse', 'atelier', 'console']);
  });

  it('defaults to pulse and clears the data-theme attribute on mount', () => {
    const { container } = render(ThemeSwitcher);
    const select = container.querySelector('select') as HTMLSelectElement;
    expect(select.value).toBe('pulse');
    expect(document.documentElement.dataset.theme).toBe('');
  });

  it('sets data-theme=atelier when the atelier option is chosen', async () => {
    const { container } = render(ThemeSwitcher);
    const select = container.querySelector('select') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'atelier' } });
    expect(select.value).toBe('atelier');
    expect(document.documentElement.dataset.theme).toBe('atelier');
  });

  it('sets data-theme=console when the console option is chosen', async () => {
    const { container } = render(ThemeSwitcher);
    const select = container.querySelector('select') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'console' } });
    expect(document.documentElement.dataset.theme).toBe('console');
  });

  it('clears data-theme when switching back to pulse from another theme', async () => {
    const { container } = render(ThemeSwitcher);
    const select = container.querySelector('select') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'console' } });
    expect(document.documentElement.dataset.theme).toBe('console');
    await fireEvent.change(select, { target: { value: 'pulse' } });
    expect(document.documentElement.dataset.theme).toBe('');
  });
});
