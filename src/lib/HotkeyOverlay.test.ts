// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W48 HotkeyOverlay shell tests — covers the W8 `?`-toggle help dialog
// with its dialog scaffolding, search-filter, Escape/close-button
// dismissal contract, and the inline focus-trap across the search field
// and the close button. Reads the canonical map via the live HOTKEYS
// import (pure data per `src/lib/data/hotkeys.ts`). `afterEach(cleanup)`
// per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { HOTKEYS } from './data/hotkeys';
import HotkeyOverlay from './HotkeyOverlay.svelte';

describe('HotkeyOverlay shell component', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders nothing when open=false', () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: false, onCloseRequest: vi.fn() },
    });
    expect(container.querySelector('.hotkey-overlay')).toBeNull();
  });

  it('renders the dialog scaffolding when open=true', () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const dialog = container.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    expect(dialog?.getAttribute('aria-label')).toBe('Operator hotkey map');
  });

  it('renders one section per HOTKEYS group with the canonical title', () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const sections = container.querySelectorAll('.hotkey-overlay__groups section');
    expect(sections.length).toBe(HOTKEYS.length);
    const titles = Array.from(sections).map((s) => s.querySelector('h3')?.textContent);
    expect(titles).toEqual(HOTKEYS.map((g) => g.title));
  });

  it('clicking the close button fires onCloseRequest', async () => {
    const onCloseRequest = vi.fn();
    const { getByLabelText } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest },
    });
    await fireEvent.click(getByLabelText('Close hotkey overlay'));
    expect(onCloseRequest).toHaveBeenCalledTimes(1);
  });

  it('Escape key on the dialog fires onCloseRequest', async () => {
    const onCloseRequest = vi.fn();
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest },
    });
    const dialog = container.querySelector('.hotkey-overlay') as HTMLElement | null;
    expect(dialog).not.toBeNull();
    if (dialog === null) return;
    await fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(onCloseRequest).toHaveBeenCalledTimes(1);
  });

  it('search input filters groups to those matching the query', async () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const search = container.querySelector('input[type="search"]') as HTMLInputElement | null;
    expect(search).not.toBeNull();
    if (search === null) return;
    await fireEvent.input(search, { target: { value: 'fullscreen' } });
    const sections = container.querySelectorAll('.hotkey-overlay__groups section');
    // "fullscreen" appears only in the Display / output group.
    expect(sections.length).toBe(1);
    expect(sections[0]?.querySelector('h3')?.textContent).toBe('Display / output');
  });

  it('shows the empty-state copy when no rows match the query', async () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const search = container.querySelector('input[type="search"]') as HTMLInputElement | null;
    expect(search).not.toBeNull();
    if (search === null) return;
    await fireEvent.input(search, { target: { value: 'zzz-no-such-hotkey' } });
    expect(container.querySelectorAll('.hotkey-overlay__groups section').length).toBe(0);
    const empty = container.querySelector('.empty');
    expect(empty?.textContent).toBe('no matches for "zzz-no-such-hotkey"');
  });

  it('renders kbd chips for each key combo plus + separators for chorded keys', () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    // The first row in the first group is { keys: ['D'], action: 'Set control to its default value' }.
    const firstRow = container.querySelector('.hotkey-overlay__groups tbody tr');
    expect(firstRow).not.toBeNull();
    const kbds = firstRow?.querySelectorAll('kbd');
    expect(kbds?.length).toBe(1);
    expect(kbds?.[0]?.textContent).toBe('D');
    // Find a chorded combo (e.g., "Cmd+R") and assert + separator.
    const dialog = container.querySelector('.hotkey-overlay') as HTMLElement;
    const hasPlus = dialog.querySelector('.plus');
    expect(hasPlus).not.toBeNull();
    expect(hasPlus?.getAttribute('aria-hidden')).toBe('true');
  });

  it('Tab from the last tabbable element triggers the focus-trap (preventDefault)', async () => {
    // Focus-trap is computed via `querySelectorAll('input, button')` which
    // returns matches in document order. The header close button comes
    // first; the search input comes after. So the close button is `first`
    // and the search input is `last`. Tab on `last` (no shift) should
    // engage the trap. We verify this via `defaultPrevented` because
    // jsdom's `fireEvent` does not preserve in-handler focus changes
    // across the dispatched-event boundary, but `event.preventDefault()`
    // is observable post-dispatch.
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const search = container.querySelector('input[type="search"]') as HTMLInputElement | null;
    const dialog = container.querySelector('.hotkey-overlay') as HTMLElement | null;
    expect(search).not.toBeNull();
    expect(dialog).not.toBeNull();
    if (search === null || dialog === null) return;
    search.focus();
    expect(document.activeElement).toBe(search);
    let captured: KeyboardEvent | null = null;
    const captor = (e: Event) => {
      captured = e as KeyboardEvent;
    };
    dialog.addEventListener('keydown', captor);
    await fireEvent.keyDown(search, { key: 'Tab' });
    dialog.removeEventListener('keydown', captor);
    expect(captured).not.toBeNull();
    expect(captured!.defaultPrevented).toBe(true);
  });

  it('Shift+Tab from the first tabbable element triggers the focus-trap (preventDefault)', async () => {
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const closeBtn = container.querySelector('header button') as HTMLButtonElement | null;
    const dialog = container.querySelector('.hotkey-overlay') as HTMLElement | null;
    expect(closeBtn).not.toBeNull();
    expect(dialog).not.toBeNull();
    if (closeBtn === null || dialog === null) return;
    closeBtn.focus();
    expect(document.activeElement).toBe(closeBtn);
    let captured: KeyboardEvent | null = null;
    const captor = (e: Event) => {
      captured = e as KeyboardEvent;
    };
    dialog.addEventListener('keydown', captor);
    await fireEvent.keyDown(closeBtn, { key: 'Tab', shiftKey: true });
    dialog.removeEventListener('keydown', captor);
    expect(captured).not.toBeNull();
    expect(captured!.defaultPrevented).toBe(true);
  });

  it('Tab from a non-edge element does not engage the focus-trap', async () => {
    // Sanity counterpart to the trap tests: when the active element is
    // neither `first` nor `last` (here we focus nothing inside the
    // dialog — the body has focus), the handler must NOT call
    // preventDefault, leaving the browser's native Tab handling to
    // proceed.
    const { container } = render(HotkeyOverlay, {
      props: { open: true, onCloseRequest: vi.fn() },
    });
    const dialog = container.querySelector('.hotkey-overlay') as HTMLElement | null;
    expect(dialog).not.toBeNull();
    if (dialog === null) return;
    document.body.focus();
    let captured: KeyboardEvent | null = null;
    const captor = (e: Event) => {
      captured = e as KeyboardEvent;
    };
    dialog.addEventListener('keydown', captor);
    await fireEvent.keyDown(dialog, { key: 'Tab' });
    dialog.removeEventListener('keydown', captor);
    expect(captured).not.toBeNull();
    expect(captured!.defaultPrevented).toBe(false);
  });
});
