// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W32 Dropdown primitive tests — exercises labels rendering,
// generated-fallback labels, and the aria-checked invariant for the
// selected option.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Dropdown from './Dropdown.svelte';

describe('Dropdown widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders one button per provided label', () => {
    const { getByRole } = render(Dropdown, {
      props: { value: 1, labels: ['warm', 'neon', 'ice', 'mono'] },
    });
    expect(getByRole('radio', { name: 'warm' })).toBeInTheDocument();
    expect(getByRole('radio', { name: 'neon' })).toBeInTheDocument();
    expect(getByRole('radio', { name: 'ice' })).toBeInTheDocument();
    expect(getByRole('radio', { name: 'mono' })).toBeInTheDocument();
  });

  it('marks only the selected index aria-checked=true', () => {
    const { getByRole } = render(Dropdown, {
      props: { value: 2, labels: ['warm', 'neon', 'ice', 'mono'] },
    });
    expect(getByRole('radio', { name: 'warm' })).toHaveAttribute('aria-checked', 'false');
    expect(getByRole('radio', { name: 'neon' })).toHaveAttribute('aria-checked', 'false');
    expect(getByRole('radio', { name: 'ice' })).toHaveAttribute('aria-checked', 'true');
    expect(getByRole('radio', { name: 'mono' })).toHaveAttribute('aria-checked', 'false');
  });

  it('falls back to "opt N" labels when only optionCount is provided', () => {
    const { getByRole } = render(Dropdown, { props: { value: 0, optionCount: 3 } });
    expect(getByRole('radio', { name: 'opt 0' })).toBeInTheDocument();
    expect(getByRole('radio', { name: 'opt 1' })).toBeInTheDocument();
    expect(getByRole('radio', { name: 'opt 2' })).toBeInTheDocument();
  });

  it('disables every option to advertise the display-only contract', () => {
    const { getByRole } = render(Dropdown, {
      props: { value: 0, labels: ['a', 'b'] },
    });
    expect(getByRole('radio', { name: 'a' })).toBeDisabled();
    expect(getByRole('radio', { name: 'b' })).toBeDisabled();
  });
});
