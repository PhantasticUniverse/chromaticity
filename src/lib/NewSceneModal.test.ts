// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W50 NewSceneModal interaction tests — covers the W22 PR3 new-scene
// modal stub: open/close gating, autofocus on the name input, Escape
// dismissal, focus-trap wrap on Tab / Shift+Tab, backdrop click
// dismissal, and Create button trimming + callback wiring.
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import NewSceneModal from './NewSceneModal.svelte';

describe('NewSceneModal shell component', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders nothing when open=false', () => {
    const { container } = render(NewSceneModal, {
      props: { open: false, onCreate: vi.fn(), onClose: vi.fn() },
    });
    expect(container.querySelector('.new-scene')).toBeNull();
    expect(container.querySelector('.new-scene__backdrop')).toBeNull();
  });

  it('renders the backdrop, dialog, name input, and Cancel/Create buttons when open=true', () => {
    const { container, getByRole, getByLabelText } = render(NewSceneModal, {
      props: { open: true, onCreate: vi.fn(), onClose: vi.fn() },
    });
    expect(container.querySelector('.new-scene__backdrop')).not.toBeNull();
    const dialog = getByRole('dialog');
    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect(dialog.getAttribute('aria-labelledby')).toBe('new-scene-title');
    expect(container.querySelector('#new-scene-title')?.textContent).toBe('New scene');
    expect(getByLabelText('Scene name')).toBeInstanceOf(HTMLInputElement);
    expect(getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(getByRole('button', { name: 'Create' })).toBeInTheDocument();
  });

  it('autofocuses the Scene name input when opened', async () => {
    const { getByLabelText } = render(NewSceneModal, {
      props: { open: true, onCreate: vi.fn(), onClose: vi.fn() },
    });
    // The component schedules focus via queueMicrotask; await it.
    await Promise.resolve();
    const input = getByLabelText('Scene name');
    expect(document.activeElement).toBe(input);
  });

  it('Escape key on the dialog fires onClose', async () => {
    const onClose = vi.fn();
    const { getByRole } = render(NewSceneModal, {
      props: { open: true, onCreate: vi.fn(), onClose },
    });
    await fireEvent.keyDown(getByRole('dialog'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('clicking the backdrop fires onClose', async () => {
    const onClose = vi.fn();
    const { container } = render(NewSceneModal, {
      props: { open: true, onCreate: vi.fn(), onClose },
    });
    const backdrop = container.querySelector('.new-scene__backdrop');
    expect(backdrop).not.toBeNull();
    if (backdrop === null) return;
    await fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('clicking Cancel fires onClose without creating', async () => {
    const onClose = vi.fn();
    const onCreate = vi.fn();
    const { getByRole } = render(NewSceneModal, {
      props: { open: true, onCreate, onClose },
    });
    await fireEvent.click(getByRole('button', { name: 'Cancel' }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onCreate).not.toHaveBeenCalled();
  });

  it('Create button passes the trimmed input value to onCreate', async () => {
    const onCreate = vi.fn();
    const { getByLabelText, getByRole } = render(NewSceneModal, {
      props: { open: true, onCreate, onClose: vi.fn() },
    });
    const input = getByLabelText('Scene name') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: '  my scene  ' } });
    await fireEvent.click(getByRole('button', { name: 'Create' }));
    expect(onCreate).toHaveBeenCalledTimes(1);
    expect(onCreate).toHaveBeenCalledWith('my scene');
  });

  it('Tab on the last button (Create) wraps focus to the first tabbable (input)', async () => {
    const { getByLabelText, getByRole } = render(NewSceneModal, {
      props: { open: true, onCreate: vi.fn(), onClose: vi.fn() },
    });
    await Promise.resolve();
    const input = getByLabelText('Scene name') as HTMLInputElement;
    const createBtn = getByRole('button', { name: 'Create' }) as HTMLButtonElement;
    createBtn.focus();
    expect(document.activeElement).toBe(createBtn);
    await fireEvent.keyDown(getByRole('dialog'), { key: 'Tab' });
    expect(document.activeElement).toBe(input);
  });

  it('Shift+Tab on the first tabbable (input) wraps focus to the last button (Create)', async () => {
    const { getByLabelText, getByRole } = render(NewSceneModal, {
      props: { open: true, onCreate: vi.fn(), onClose: vi.fn() },
    });
    await Promise.resolve();
    const input = getByLabelText('Scene name') as HTMLInputElement;
    const createBtn = getByRole('button', { name: 'Create' }) as HTMLButtonElement;
    expect(document.activeElement).toBe(input);
    await fireEvent.keyDown(getByRole('dialog'), { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(createBtn);
  });
});
