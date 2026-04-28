// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W47 StatusBar shell tests — covers the W15 PR2 status bar's hint
// copy, right-side metric placeholders, console-toggle button, and
// version chip. Console store is exercised live (it's a runes-backed
// singleton); the test toggles it via the rendered button + asserts
// the aria-pressed reflection. `afterEach(cleanup)` per
// `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import StatusBar from './StatusBar.svelte';
import { consoleStore } from './stores/console.svelte';

describe('StatusBar shell component', () => {
  afterEach(() => {
    if (consoleStore.isOpen) consoleStore.toggle();
    cleanup();
  });

  it('renders the hint copy with the hotkey reference', () => {
    const { getByText } = render(StatusBar);
    expect(getByText(/Hover any control to see what it does/)).toBeInTheDocument();
  });

  it('renders an em-dash placeholder for unwired CPU / GPU / MEM metrics', () => {
    const { container } = render(StatusBar);
    const values = container.querySelectorAll('.status-bar__metric-value');
    expect(values.length).toBe(3); // CPU, GPU, MEM
    values.forEach((node) => expect(node.textContent?.trim()).toBe('—'));
  });

  it('renders zero counters for warnings and errors', () => {
    const { container } = render(StatusBar);
    const counters = container.querySelectorAll('.status-bar__counter');
    expect(counters.length).toBe(2);
    expect(counters[0]?.textContent).toContain('0 warnings');
    expect(counters[1]?.textContent).toContain('0 errors');
    expect(counters[0]?.getAttribute('data-state')).toBe('idle');
    expect(counters[1]?.getAttribute('data-state')).toBe('idle');
  });

  it('renders the Console toggle with aria-pressed=false by default', () => {
    const { container } = render(StatusBar);
    const btn = container.querySelector('.status-bar__console');
    expect(btn?.getAttribute('aria-pressed')).toBe('false');
    expect(btn?.textContent).toContain('Console');
  });

  it('flips aria-pressed when the Console button toggles the store', async () => {
    const { container } = render(StatusBar);
    const btn = container.querySelector('.status-bar__console') as HTMLButtonElement;
    expect(btn.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(btn);
    expect(btn.getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(btn);
    expect(btn.getAttribute('aria-pressed')).toBe('false');
  });

  it('renders a version chip with a v-prefixed semver placeholder', () => {
    const { container } = render(StatusBar);
    const version = container.querySelector('.status-bar__version');
    expect(version?.textContent).toMatch(/^v\d+\.\d+\.\d+$/);
  });

  it('marks the footer with the Status bar aria-label', () => {
    const { container } = render(StatusBar);
    const footer = container.querySelector('footer.status-bar');
    expect(footer?.getAttribute('aria-label')).toBe('Status bar');
  });
});
