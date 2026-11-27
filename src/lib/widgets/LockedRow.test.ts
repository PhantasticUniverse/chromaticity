// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W34b LockedRow primitive tests — exercise the plain-label branch,
// the tooltip / aria-disabled invariants, and the onNavigate
// click + keyboard activation contract.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import LockedRow from './LockedRow.svelte';

describe('LockedRow widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the label text and the lock svg', () => {
    const { getByRole, container } = render(LockedRow, {
      props: { label: 'NDI Input', capability: 'io.ndi.input' },
    });
    expect(getByRole('button')).toHaveTextContent('NDI Input');
    expect(container.querySelector('.locked-row__lock')).not.toBeNull();
  });

  it('exposes the capability in the tooltip + aria-label', () => {
    const { getByRole } = render(LockedRow, {
      props: { label: 'NDI Output', capability: 'io.ndi.output' },
    });
    const btn = getByRole('button');
    expect(btn).toHaveAttribute('title', 'Requires io.ndi.output. See Settings → License.');
    expect(btn).toHaveAttribute('aria-label', 'Locked: requires io.ndi.output');
    expect(btn).toHaveAttribute('aria-disabled', 'true');
  });

  it('fires onNavigate on click', async () => {
    const onNavigate = vi.fn();
    const { getByRole } = render(LockedRow, {
      props: { label: 'X', capability: 'cap.x', onNavigate },
    });
    await fireEvent.click(getByRole('button'));
    expect(onNavigate).toHaveBeenCalledTimes(1);
  });

  it('fires onNavigate on Enter and Space keypress', async () => {
    const onNavigate = vi.fn();
    const { getByRole } = render(LockedRow, {
      props: { label: 'X', capability: 'cap.x', onNavigate },
    });
    const btn = getByRole('button');
    await fireEvent.keyDown(btn, { key: 'Enter' });
    expect(onNavigate).toHaveBeenCalledTimes(1);
    await fireEvent.keyDown(btn, { key: ' ' });
    expect(onNavigate).toHaveBeenCalledTimes(2);
  });

  it('ignores other keys', async () => {
    const onNavigate = vi.fn();
    const { getByRole } = render(LockedRow, {
      props: { label: 'X', capability: 'cap.x', onNavigate },
    });
    await fireEvent.keyDown(getByRole('button'), { key: 'Tab' });
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('renders without onNavigate without throwing', () => {
    const { getByRole } = render(LockedRow, {
      props: { label: 'X', capability: 'cap.x' },
    });
    // Click should be a no-op rather than a TypeError.
    expect(() => getByRole('button').click()).not.toThrow();
  });
});
