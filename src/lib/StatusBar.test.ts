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

  // Wave 2.3 Lane Q.2 (G.14) — the W15 PR2 em-dash placeholders are
  // replaced with bundle-faithful fixed values (CPU 12% / GPU 42%)
  // plus a `performance.memory`-derived approximation for MEM
  // (jsdom omits `performance.memory`, so the readout falls back to
  // the bundle's `2.1 GB` fixture string in this test environment).
  it('renders bundle-faithful CPU / GPU / MEM placeholders plus the PEAK dBFS chip', () => {
    const { container } = render(StatusBar);
    const values = container.querySelectorAll('.status-bar__metric-value');
    expect(values.length).toBe(4); // CPU, GPU, MEM, PEAK (DESIGN.md bundle alignment 2026-05-18)
    expect(values[0]?.textContent?.trim()).toBe('12%');
    expect(values[1]?.textContent?.trim()).toBe('42%');
    // jsdom does not expose performance.memory → MEM falls back to
    // the bundle's `2.1` fixture string.
    expect(values[2]?.textContent?.trim()).toBe('2.1');
    // PEAK shows the em-dash placeholder under no live audio snapshot.
    expect(values[3]?.textContent?.trim()).toBe('—');
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

  // ---------------- DESIGN.md "Bundle alignment" — audio dBFS chip ---------

  it('renders the PEAK dBFS chip with an em-dash placeholder when no live audio snapshot', () => {
    const { container } = render(StatusBar);
    const peak = container.querySelector('[data-testid="status-bar-peak-dbfs"]');
    expect(peak).not.toBeNull();
    expect(peak?.textContent).toBe('—');
  });
});
