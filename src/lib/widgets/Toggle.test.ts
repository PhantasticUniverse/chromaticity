// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W37 Toggle widget tests — extended in Wave 2.1 Lane A.2 to cover
// click → `onchange(!value)` dispatch. The W8 baseline contract (on /
// off class flip, orientation switch, label render) is preserved; the
// W2.1 A.2 axis (click interactivity) gates on the optional `onchange`
// callback presence. When `onchange` is provided the widget renders
// as a `role="switch"` button (keyboard activatable); without it the
// widget stays a presentational `<div>` matching the original W8
// surface so display-only callers (Inputs.svelte) are unchanged.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import Toggle from './Toggle.svelte';

describe('Toggle widget', () => {
  afterEach(() => {
    cleanup();
  });

  // ---------------- W8 display contract (preserved) ----------------------

  it('renders the switch structure (display-only)', () => {
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

  it('omits the label DOM when no label prop is given (display-only)', () => {
    const { container } = render(Toggle, { props: { value: false } });
    expect(container.querySelector('.tog-label')).toBeNull();
  });

  it('renders as a presentational div without onchange', () => {
    const { container } = render(Toggle, { props: { value: false } });
    expect(container.querySelector('button.tog')).toBeNull();
    const root = container.querySelector('.tog') as HTMLElement | null;
    expect(root?.tagName).toBe('DIV');
  });

  // ---------------- Wave 2.1 Lane A.2 interactivity ---------------------

  it('renders as a role="switch" button when onchange is provided', () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: false, onchange } });
    const btn = container.querySelector('button.tog');
    expect(btn).not.toBeNull();
    expect(btn?.getAttribute('role')).toBe('switch');
    expect(btn?.getAttribute('aria-checked')).toBe('false');
  });

  it('aria-checked reflects value=true', () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: true, onchange, label: 'mute' } });
    const btn = container.querySelector('button.tog');
    expect(btn?.getAttribute('aria-checked')).toBe('true');
    expect(btn?.getAttribute('aria-label')).toBe('mute');
  });

  it('clicking the switch fires onchange with the negated value', async () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: false, onchange } });
    const btn = container.querySelector('button.tog') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    if (btn === null) return;
    await fireEvent.click(btn);
    expect(onchange).toHaveBeenCalledWith(true);
  });

  it('clicking again fires onchange with the negated value (true → false)', async () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: true, onchange } });
    const btn = container.querySelector('button.tog') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    if (btn === null) return;
    await fireEvent.click(btn);
    expect(onchange).toHaveBeenCalledWith(false);
  });

  it('Space key activates the switch (keyboard accessibility)', async () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: false, onchange } });
    const btn = container.querySelector('button.tog') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    if (btn === null) return;
    await fireEvent.keyDown(btn, { key: ' ' });
    expect(onchange).toHaveBeenCalledWith(true);
  });

  it('Enter key activates the switch (keyboard accessibility)', async () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: false, onchange } });
    const btn = container.querySelector('button.tog') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    if (btn === null) return;
    await fireEvent.keyDown(btn, { key: 'Enter' });
    expect(onchange).toHaveBeenCalledWith(true);
  });

  it('disabled=true suppresses onchange and adds disabled class', async () => {
    const onchange = vi.fn();
    const { container } = render(Toggle, { props: { value: false, onchange, disabled: true } });
    const btn = container.querySelector('button.tog') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    if (btn === null) return;
    expect(btn.classList.contains('tog--disabled')).toBe(true);
    expect(btn.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(btn);
    expect(onchange).not.toHaveBeenCalled();
  });
});
