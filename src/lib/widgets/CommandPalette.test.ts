// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W46 CommandPalette interaction tests — Wave-29 PR1 widget; bundle source
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/extras.jsx` lines 124-166.
// The Cmd+K toggle lives in AppShell, not the widget; the component only
// owns Escape / backdrop / close-button dismiss and search filtering.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { DEFAULT_COMMAND_PALETTE_ACTIONS } from './command-palette.fixtures';
import CommandPalette from './CommandPalette.svelte';

describe('CommandPalette widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders nothing when open=false', () => {
    const { container } = render(CommandPalette, {
      props: { open: false, onClose: vi.fn() },
    });
    expect(container.querySelector('.command-palette__backdrop')).toBeNull();
    expect(container.querySelector('.command-palette')).toBeNull();
  });

  it('renders the backdrop + dialog + default seven actions when open=true', () => {
    const { container, getAllByRole } = render(CommandPalette, {
      props: { open: true, onClose: vi.fn() },
    });
    expect(container.querySelector('.command-palette__backdrop')).not.toBeNull();
    const dialog = container.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    expect(dialog?.getAttribute('aria-label')).toBe('Command palette');
    // Each action renders as a button with class `command-palette__row`.
    const rows = container.querySelectorAll('.command-palette__row');
    expect(rows.length).toBe(DEFAULT_COMMAND_PALETTE_ACTIONS.length);
    // Close button + N action rows = N+1 buttons.
    expect(getAllByRole('button').length).toBe(DEFAULT_COMMAND_PALETTE_ACTIONS.length + 1);
  });

  it('Escape key on the backdrop fires onClose', async () => {
    const onClose = vi.fn();
    const { container } = render(CommandPalette, { props: { open: true, onClose } });
    const backdrop = container.querySelector('.command-palette__backdrop');
    expect(backdrop).not.toBeNull();
    if (backdrop === null) return;
    await fireEvent.keyDown(backdrop, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('clicking the close button fires onClose', async () => {
    const onClose = vi.fn();
    const { getByLabelText } = render(CommandPalette, { props: { open: true, onClose } });
    const closeBtn = getByLabelText('Close command palette');
    await fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('clicking the backdrop (target === currentTarget) fires onClose', async () => {
    const onClose = vi.fn();
    const { container } = render(CommandPalette, { props: { open: true, onClose } });
    const backdrop = container.querySelector('.command-palette__backdrop') as HTMLElement | null;
    expect(backdrop).not.toBeNull();
    if (backdrop === null) return;
    // fireEvent.click dispatches with target === the element it's fired on,
    // so the backdrop handler's target/currentTarget identity check passes.
    await fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('selecting an action invokes the action.onSelect callback and onClose', async () => {
    const onClose = vi.fn();
    const onSelect = vi.fn();
    const commands = [
      { id: 'test-action', label: 'Test action', meta: 'meta · context', onSelect },
    ] as const;
    const { container } = render(CommandPalette, {
      props: { open: true, onClose, commands },
    });
    const row = container.querySelector('.command-palette__row') as HTMLButtonElement | null;
    expect(row).not.toBeNull();
    if (row === null) return;
    await fireEvent.click(row);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('search input filters action rows by label and meta substring', async () => {
    const { container } = render(CommandPalette, {
      props: { open: true, onClose: vi.fn() },
    });
    const search = container.querySelector('.command-palette__search') as HTMLInputElement | null;
    expect(search).not.toBeNull();
    if (search === null) return;
    // "snap" matches both `snap-recall` and `snap-save` rows (id-by-substring
    // via label "Snap" / meta "perform · /favslots/{n}…").
    await fireEvent.input(search, { target: { value: 'snap' } });
    const rows = container.querySelectorAll('.command-palette__row');
    expect(rows.length).toBe(2);
    expect(rows[0]?.textContent).toMatch(/Snap/);
  });

  it('shows the empty-state copy when no commands match the query', async () => {
    const { container } = render(CommandPalette, {
      props: { open: true, onClose: vi.fn() },
    });
    const search = container.querySelector('.command-palette__search') as HTMLInputElement | null;
    expect(search).not.toBeNull();
    if (search === null) return;
    await fireEvent.input(search, { target: { value: 'zzz-no-such-command' } });
    expect(container.querySelectorAll('.command-palette__row').length).toBe(0);
    expect(container.querySelector('.command-palette__empty')?.textContent).toBe(
      'no matching command',
    );
  });
});
