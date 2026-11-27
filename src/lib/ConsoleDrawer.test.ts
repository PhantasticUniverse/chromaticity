// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W52 ConsoleDrawer shell tests — exercises the W21 PR4 global console
// drawer: open/close gating off `consoleStore.isOpen`, autofocus on the
// search input, Escape + backdrop + close-button dismissal, severity
// chip / source dropdown / search filtering against the 30-row fixture
// corpus, pause toggle, Clear-filters reset, focus-trap on Tab, and
// the per-row Copy button's clipboard write + "Copied" badge swap.
//
// `consoleStore` is a runes-backed singleton; `afterEach` resets every
// flag back to defaults so cross-test state cannot leak. `cleanup()`
// per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import ConsoleDrawer from './ConsoleDrawer.svelte';
import { CONSOLE_FIXTURES } from './data/console-fixtures';
import { consoleStore } from './stores/console.svelte';

function resetConsoleStore(): void {
  consoleStore.isOpen = false;
  consoleStore.isPaused = false;
  consoleStore.severityFilter.clear();
  consoleStore.sourceFilter = 'all';
  consoleStore.query = '';
}

describe('ConsoleDrawer shell component', () => {
  afterEach(() => {
    resetConsoleStore();
    cleanup();
    vi.unstubAllGlobals();
  });

  it('renders nothing when consoleStore.isOpen is false', () => {
    const { container } = render(ConsoleDrawer);
    expect(container.querySelector('.console-drawer')).toBeNull();
    expect(container.querySelector('.console-drawer__backdrop')).toBeNull();
  });

  it('renders the backdrop, dialog, and one row per fixture entry when open', () => {
    consoleStore.isOpen = true;
    const { container, getByRole } = render(ConsoleDrawer);
    expect(container.querySelector('.console-drawer__backdrop')).not.toBeNull();
    const dialog = getByRole('dialog');
    expect(dialog.getAttribute('aria-modal')).toBe('false');
    expect(dialog.getAttribute('aria-label')).toBe('Diagnostics console');
    const rows = container.querySelectorAll('.console-drawer__row');
    expect(rows.length).toBe(CONSOLE_FIXTURES.length);
  });

  it('autofocuses the search input on open', async () => {
    consoleStore.isOpen = true;
    const { getByLabelText } = render(ConsoleDrawer);
    // Focus is scheduled via queueMicrotask; await it.
    await Promise.resolve();
    const search = getByLabelText('Search console entries');
    expect(document.activeElement).toBe(search);
  });

  it('renders the live badge by default and flips to paused when togglePause runs', async () => {
    consoleStore.isOpen = true;
    const { container } = render(ConsoleDrawer);
    const live = container.querySelector('.console-drawer__live');
    expect(live?.getAttribute('data-state')).toBe('live');
    expect(live?.textContent?.trim()).toBe('live');

    const pauseBtn = container.querySelectorAll('.console-drawer__action')[0] as HTMLButtonElement;
    expect(pauseBtn?.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(pauseBtn);
    expect(consoleStore.isPaused).toBe(true);
    expect(live?.getAttribute('data-state')).toBe('paused');
    expect(live?.textContent?.trim()).toBe('paused');
    expect(pauseBtn?.getAttribute('aria-pressed')).toBe('true');
  });

  it('Escape key closes the drawer (document-level listener)', async () => {
    consoleStore.isOpen = true;
    render(ConsoleDrawer);
    expect(consoleStore.isOpen).toBe(true);
    await fireEvent.keyDown(document, { key: 'Escape' });
    expect(consoleStore.isOpen).toBe(false);
  });

  it('clicking the backdrop closes the drawer', async () => {
    consoleStore.isOpen = true;
    const { container } = render(ConsoleDrawer);
    const backdrop = container.querySelector('.console-drawer__backdrop') as HTMLElement | null;
    expect(backdrop).not.toBeNull();
    if (backdrop === null) return;
    await fireEvent.click(backdrop);
    expect(consoleStore.isOpen).toBe(false);
  });

  it('clicking the Close button closes the drawer', async () => {
    consoleStore.isOpen = true;
    const { getByLabelText } = render(ConsoleDrawer);
    const closeBtn = getByLabelText('Close console drawer');
    await fireEvent.click(closeBtn);
    expect(consoleStore.isOpen).toBe(false);
  });

  it('toggling a severity chip narrows the visible rows to that severity', async () => {
    consoleStore.isOpen = true;
    const { container, getByRole } = render(ConsoleDrawer);
    const errorChip = getByRole('button', { name: 'Error' });
    expect(errorChip.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(errorChip);
    expect(errorChip.getAttribute('aria-pressed')).toBe('true');
    const rows = container.querySelectorAll('.console-drawer__row');
    // Two error fixtures in the corpus.
    expect(rows.length).toBe(2);
    rows.forEach((row) => expect(row.getAttribute('data-severity')).toBe('error'));
  });

  it('the All chip clears any active severity filter', async () => {
    consoleStore.isOpen = true;
    consoleStore.severityFilter.add('error');
    const { container, getByRole } = render(ConsoleDrawer);
    expect(container.querySelectorAll('.console-drawer__row').length).toBe(2);
    const allChip = getByRole('button', { name: 'All' });
    expect(allChip.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(allChip);
    expect(allChip.getAttribute('aria-pressed')).toBe('true');
    expect(consoleStore.severityFilter.size).toBe(0);
    expect(container.querySelectorAll('.console-drawer__row').length).toBe(CONSOLE_FIXTURES.length);
  });

  it('the source dropdown narrows rows to a single source view', async () => {
    consoleStore.isOpen = true;
    const { container, getByLabelText } = render(ConsoleDrawer);
    const select = getByLabelText('Filter by source view') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'midi' } });
    expect(consoleStore.sourceFilter).toBe('midi');
    const rows = container.querySelectorAll('.console-drawer__row');
    // Four midi fixtures in the corpus.
    expect(rows.length).toBe(4);
  });

  it('the search query filters by case-insensitive substring match on message + source', async () => {
    consoleStore.isOpen = true;
    const { container, getByLabelText } = render(ConsoleDrawer);
    const search = getByLabelText('Search console entries') as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'Launchpad' } });
    const rows = container.querySelectorAll('.console-drawer__row');
    // Two Launchpad mentions in the corpus.
    expect(rows.length).toBe(2);
    rows.forEach((row) => expect(row.textContent).toMatch(/Launchpad/i));
  });

  it('renders the empty-state copy when no entries match the current filters', async () => {
    consoleStore.isOpen = true;
    const { container, getByLabelText } = render(ConsoleDrawer);
    const search = getByLabelText('Search console entries') as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'zzz-no-such-message' } });
    expect(container.querySelectorAll('.console-drawer__row').length).toBe(0);
    const empty = container.querySelector('.console-drawer__empty');
    expect(empty?.textContent).toBe('No entries match the current filters.');
  });

  it('the Clear button resets severity, source, and query filters', async () => {
    consoleStore.isOpen = true;
    consoleStore.severityFilter.add('warn');
    consoleStore.sourceFilter = 'midi';
    consoleStore.query = 'handshake';
    const { getByRole } = render(ConsoleDrawer);
    const clearBtn = getByRole('button', { name: /Clear/ });
    await fireEvent.click(clearBtn);
    expect(consoleStore.severityFilter.size).toBe(0);
    expect(consoleStore.sourceFilter).toBe('all');
    expect(consoleStore.query).toBe('');
  });

  it('Shift+Tab on the first tabbable wraps focus to the last tabbable in the drawer', async () => {
    consoleStore.isOpen = true;
    const { getByRole, getByLabelText } = render(ConsoleDrawer);
    const dialog = getByRole('dialog');
    // Flush the queueMicrotask-scheduled autofocus on the search input
    // BEFORE we manually move focus, otherwise the microtask resolves
    // during the next `await fireEvent.keyDown` and steals focus.
    await Promise.resolve();
    // First tabbable in DOM order is the Pause button at the top of the
    // header action row (selector is `button, input, select`).
    const pauseBtn = getByRole('button', { name: /Pause/ }) as HTMLButtonElement;
    pauseBtn.focus();
    expect(document.activeElement).toBe(pauseBtn);
    await fireEvent.keyDown(dialog, { key: 'Tab', shiftKey: true });
    // The last tabbable is the final per-row Copy button.
    const copyButtons = document.querySelectorAll('.console-drawer__copy');
    const lastCopy = copyButtons[copyButtons.length - 1];
    expect(document.activeElement).toBe(lastCopy);
    // Sanity: search input was the autofocus target but the trap
    // wrap moved past it to the very last tabbable.
    expect(document.activeElement).not.toBe(getByLabelText('Search console entries'));
  });

  it('clicking a Copy button writes the formatted entry text and surfaces the Copied badge', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    consoleStore.isOpen = true;
    // Narrow to a single fixture so the first .console-drawer__copy
    // maps to a known entry — the first fixture is `lib-scan-1`.
    consoleStore.severityFilter.add('scan');
    consoleStore.sourceFilter = 'library';
    const { container } = render(ConsoleDrawer);
    const rows = container.querySelectorAll('.console-drawer__row');
    expect(rows.length).toBe(2); // two library scan rows
    const firstCopy = container.querySelector('.console-drawer__copy') as HTMLButtonElement;
    await fireEvent.click(firstCopy);
    expect(writeText).toHaveBeenCalledTimes(1);
    const written = writeText.mock.calls[0]?.[0] as string;
    expect(written).toContain('13:24:02');
    expect(written).toContain('scan');
    expect(written).toContain('library');
    expect(written).toContain('Indexing scenes');
    // Wait for the post-await microtask that flips `copiedId`.
    await Promise.resolve();
    expect(firstCopy.querySelector('.console-drawer__copied')?.textContent).toBe('Copied');
  });
});
