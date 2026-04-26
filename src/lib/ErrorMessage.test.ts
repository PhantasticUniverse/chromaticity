// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// First Vitest spec in repo history; doubles as scaffolding validation
// for the test framework and as a contract for the two render branches
// of `ErrorMessage.svelte`. Vitest globals are disabled in
// `vitest.config.ts`; we import `describe` / `it` / `expect` explicitly.

import '@testing-library/jest-dom/vitest';
import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import ErrorMessage from './ErrorMessage.svelte';

describe('ErrorMessage', () => {
  it('renders nothing when error is null', () => {
    const { container } = render(ErrorMessage, { props: { error: null } });
    expect(container.querySelector('p')).toBeNull();
  });

  it('renders the error text when error is non-empty', () => {
    const { getByRole } = render(ErrorMessage, {
      props: { error: 'something broke' },
    });
    const alert = getByRole('alert');
    expect(alert).toHaveTextContent('something broke');
  });
});
