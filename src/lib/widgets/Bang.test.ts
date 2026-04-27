// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W32 Bang primitive tests — exercises the trigger / counter mode
// labelling and the disabled display-only contract.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Bang from './Bang.svelte';

describe('Bang widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders "TRIGGER" by default in trigger mode', () => {
    const { getByRole } = render(Bang);
    const btn = getByRole('button');
    expect(btn).toHaveTextContent('TRIGGER');
    expect(btn).toBeDisabled();
  });

  it('renders "CLICK +" in counter mode plus the count pill', () => {
    const { getByRole, getByText } = render(Bang, { props: { mode: 'counter', count: 7 } });
    expect(getByRole('button')).toHaveTextContent('CLICK +');
    expect(getByText('×7')).toBeInTheDocument();
  });

  it('honours an explicit label override', () => {
    const { getByRole } = render(Bang, { props: { label: 'flash', mode: 'trigger' } });
    expect(getByRole('button')).toHaveTextContent('flash');
  });

  it('omits the count pill in trigger mode', () => {
    const { queryByText } = render(Bang, { props: { mode: 'trigger', count: 5 } });
    expect(queryByText('×5')).toBeNull();
  });
});
